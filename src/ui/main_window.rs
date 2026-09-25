use std::ops::Range;

use gpui::{
    canvas, div, point, prelude::*, px, rgba, Animation, AnimationExt, AnyElement, AppContext,
    Bounds, ClickEvent, Context, Div, DragMoveEvent, ElementInputHandler, Entity,
    EntityInputHandler, FocusHandle, FontWeight, KeyDownEvent, Pixels, Point, Render, ScrollHandle,
    Timer, UTF16Selection, Window, WindowControlArea,
};
use std::time::{Duration, Instant};

use crate::app::{
    AppState, OperationState, PendingAction, ServiceFilter, ServiceStatus, Theme, View,
};
use crate::backend::{self, ServiceOperation};
use crate::cleanup::{self, CleanupCategory, DeleteMode};
use crate::i18n::Language;
use crate::theme::{StatusColors, ThemeColors};
use crate::ui::{components, input};
use app_service::StartType;

const TOOLS: [(&str, &str); 1] = [("tool_sysinfo", "tool_sysinfo_desc")];

const SERVICE_NAME_COLUMN_WIDTH: f32 = 180.;
const SERVICE_DISPLAY_COLUMN_WIDTH: f32 = 200.;
const SERVICE_STATUS_COLUMN_WIDTH: f32 = 96.;
const SERVICE_ACTIONS_COLUMN_WIDTH: f32 = 210.;

const ICON_SERVICE: &str = "▤";
const ICON_TOOLS: &str = "▦";
const ICON_CLEANUP: &str = "▧";
const ICON_SCRIPTS: &str = "⟳";
const ICON_SETTINGS: &str = "⚙︎";

pub struct MainWindow {
    pub(crate) state: Entity<AppState>,
    pub(crate) focus_handle: FocusHandle,
    search_focus: FocusHandle,
    service_search_focus: FocusHandle,
    service_scroll: ScrollHandle,
    dialog_scroll: ScrollHandle,
    sysinfo_scroll: ScrollHandle,
    /// Managed service awaiting delete confirmation.
    pending_delete: Option<String>,
    /// Cleanup confirmation dialogs.
    pending_clean: bool,
    pending_empty_bin: bool,
    /// Whether a first refresh has already happened, so the launch refresh does
    /// not show the completion notice.
    refresh_notice_ready: bool,
}

/// Payload carried while dragging a custom scrollbar thumb.
#[derive(Clone)]
struct ScrollbarDrag {
    handle: ScrollHandle,
}

/// Invisible preview view for the scrollbar drag interaction.
struct ScrollbarDragPreview;

/// Parameters for the shared confirmation dialog.
struct ConfirmDialog {
    title: &'static str,
    message: String,
    ok_label: &'static str,
    ok_id: &'static str,
    cancel_id: &'static str,
    danger: bool,
    on_ok: fn(&mut MainWindow, &ClickEvent, &mut Window, &mut Context<MainWindow>),
    on_cancel: fn(&mut MainWindow, &ClickEvent, &mut Window, &mut Context<MainWindow>),
}

impl Render for ScrollbarDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl MainWindow {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        Self {
            state,
            focus_handle: cx.focus_handle(),
            search_focus: cx.focus_handle(),
            service_search_focus: cx.focus_handle(),
            service_scroll: ScrollHandle::new(),
            dialog_scroll: ScrollHandle::new(),
            sysinfo_scroll: ScrollHandle::new(),
            pending_delete: None,
            pending_clean: false,
            pending_empty_bin: false,
            refresh_notice_ready: false,
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // Text insertion is owned by the platform input handler (WM_CHAR / IME).
        // Backspace is filtered out of WM_CHAR, so handle it here for whichever
        // text field currently has focus.
        if input::text_editing_input(event) != Some(input::TextEditingInput::Backspace) {
            return;
        }
        if self.state.read(cx).show_add_dialog() {
            self.update_state(cx, |state| state.add_dialog_backspace());
        } else if self.service_search_focus.is_focused(window) {
            self.update_state(cx, |state| state.service_search_backspace());
        }
    }

    fn minimize_window_clicked(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        window.minimize_window();
    }

    fn maximize_window_clicked(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        toggle_window_zoom(window);
    }

    fn close_window_clicked(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        window.remove_window();
    }

    fn update_state(&self, cx: &mut Context<Self>, update: impl FnOnce(&mut AppState) + 'static) {
        self.state.update(cx, |state, cx| {
            update(state);
            cx.notify();
        });
    }

    fn select_view(
        &mut self,
        view: View,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, move |state| state.set_view(view));
    }

    fn select_tool(
        &mut self,
        index: usize,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, move |state| state.select_tool(index));
    }

    fn select_dialog_service(
        &mut self,
        index: usize,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, move |state| state.select_add_dialog(index));
    }

    fn set_language_zh(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.update_state(cx, |state| state.set_language(Language::Chinese));
    }

    fn set_language_en(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.update_state(cx, |state| state.set_language(Language::English));
    }

    fn set_theme_dark(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.update_state(cx, |state| state.set_theme(Theme::Dark));
    }

    fn set_theme_light(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.update_state(cx, |state| state.set_theme(Theme::Light));
    }

    fn begin_add_dialog(&mut self, cx: &mut Context<Self>) {
        // Fresh handle so the custom scrollbar re-measures for this dialog.
        self.dialog_scroll = ScrollHandle::new();
        self.update_state(cx, |state| state.begin_add_dialog());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let services = cx
                .background_spawn(async { backend::list_services() })
                .await;
            state
                .update(cx, |state, cx| {
                    state.set_add_dialog_services(services);
                    cx.notify();
                })
                .ok();
            // The custom scrollbar reads its metrics after the list has been
            // laid out, so it needs one more frame to appear.
            Timer::after(Duration::from_millis(100)).await;
            state.update(cx, |_state, cx| cx.notify()).ok();
        })
        .detach();
    }

    pub(crate) fn refresh_services(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).operation_state() == OperationState::Refreshing {
            return;
        }
        let action = self.state.update(cx, |state, cx| {
            let action = state.request_refresh();
            cx.notify();
            action
        });
        // Announce manual refreshes, but not the automatic one on launch.
        let announce = self.refresh_notice_ready;
        self.refresh_notice_ready = true;
        self.spawn_service_operation(action, announce, cx);
    }

    fn start_service(&mut self, name: String, cx: &mut Context<Self>) {
        if self.state.read(cx).current_view() != View::Service {
            return;
        }
        let action = self.state.update(cx, move |state, cx| {
            let action = state.request_start_service(&name);
            cx.notify();
            action
        });
        if let Some(action) = action {
            self.spawn_service_operation(action, false, cx);
        }
    }

    fn stop_service(&mut self, name: String, cx: &mut Context<Self>) {
        if self.state.read(cx).current_view() != View::Service {
            return;
        }
        let action = self.state.update(cx, move |state, cx| {
            let action = state.request_stop_service(&name);
            cx.notify();
            action
        });
        if let Some(action) = action {
            self.spawn_service_operation(action, false, cx);
        }
    }

    fn start_all(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).current_view() == View::Service {
            let action = self.state.update(cx, |state, cx| {
                let action = state.request_start_all();
                cx.notify();
                action
            });
            self.spawn_service_operation(action, false, cx);
        }
    }

    fn stop_all(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).current_view() == View::Service {
            let action = self.state.update(cx, |state, cx| {
                let action = state.request_stop_all();
                cx.notify();
                action
            });
            self.spawn_service_operation(action, false, cx);
        }
    }

    fn spawn_service_operation(
        &mut self,
        action: PendingAction,
        announce: bool,
        cx: &mut Context<Self>,
    ) {
        let is_refresh = matches!(action, PendingAction::RefreshAll(_));
        let operation = match action {
            PendingAction::StartService(name) => ServiceOperation::Start(name),
            PendingAction::StopService(name) => ServiceOperation::Stop(name),
            PendingAction::StartAll(names) => ServiceOperation::StartAll(names),
            PendingAction::StopAll(names) => ServiceOperation::StopAll(names),
            PendingAction::RefreshAll(names) => ServiceOperation::RefreshAll(names),
        };
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let started = Instant::now();
            let results = cx
                .background_spawn(async move { backend::execute_service_operation(operation) })
                .await;

            // A refresh can finish in a few milliseconds with the native backend;
            // keep it visible long enough to register as an interaction.
            if is_refresh {
                let minimum = Duration::from_millis(700);
                let elapsed = started.elapsed();
                if elapsed < minimum {
                    Timer::after(minimum - elapsed).await;
                }
            }

            let refreshed = results.len();
            let pending = state
                .update(cx, |state, cx| {
                    for result in results {
                        match result.status {
                            Ok(status) => state.apply_service_success(&result.name, &status),
                            Err(error) => state.apply_service_error(&result.name, &error),
                        }
                    }
                    if state.operation_state() != OperationState::Error {
                        state.set_operation_state(OperationState::Idle);
                    }
                    if is_refresh && announce {
                        let notice = state.t().refresh_done.replace("{}", &refreshed.to_string());
                        state.set_refresh_notice(notice);
                    }
                    let pending: Vec<String> = state
                        .managed_services()
                        .iter()
                        .map(|service| service.name.clone())
                        .filter(|name| {
                            matches!(
                                state.service_status(name),
                                ServiceStatus::Starting | ServiceStatus::Stopping
                            )
                        })
                        .collect();
                    cx.notify();
                    pending
                })
                .unwrap_or_default();

            Timer::after(Duration::from_millis(100)).await;
            state.update(cx, |_state, cx| cx.notify()).ok();

            if is_refresh && announce {
                Timer::after(Duration::from_millis(1100)).await;
                state
                    .update(cx, |state, cx| {
                        state.clear_refresh_notice();
                        cx.notify();
                    })
                    .ok();
            }

            if pending.is_empty() {
                return;
            }

            // The native service API returns as soon as the request is accepted,
            // so poll in the background and update the UI as services settle.
            for _ in 0..75 {
                Timer::after(Duration::from_millis(400)).await;
                let names = pending.clone();
                let statuses = cx
                    .background_spawn(async move {
                        names
                            .into_iter()
                            .map(|name| {
                                let status = backend::service_status(&name);
                                (name, status)
                            })
                            .collect::<Vec<_>>()
                    })
                    .await;
                let settled = state
                    .update(cx, |state, cx| {
                        let mut settled = true;
                        for (name, status) in statuses {
                            match status {
                                Ok(status) => state.apply_service_success(&name, &status),
                                Err(error) => state.apply_service_error(&name, &error),
                            }
                            if matches!(
                                state.service_status(&name),
                                ServiceStatus::Starting | ServiceStatus::Stopping
                            ) {
                                settled = false;
                            }
                        }
                        cx.notify();
                        settled
                    })
                    .unwrap_or(true);
                if settled {
                    break;
                }
            }
        })
        .detach();
    }

    fn open_tool(&mut self, cx: &mut Context<Self>) {
        let opened = self.state.update(cx, |state, cx| {
            let opened = state.open_tool_detail();
            cx.notify();
            opened
        });
        if opened && self.state.read(cx).system_info_loading() {
            let state = self.state.clone();
            cx.spawn(async move |_this, cx| {
                let result = cx
                    .background_spawn(async { backend::fetch_system_info() })
                    .await;
                state
                    .update(cx, |state, cx| {
                        state.set_system_info(result);
                        cx.notify();
                    })
                    .ok();
                Timer::after(Duration::from_millis(100)).await;
                state.update(cx, |_state, cx| cx.notify()).ok();
            })
            .detach();
        }
    }

    fn run_script(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).scripts_selected() != 0 {
            return;
        }
        self.update_state(cx, |state| state.begin_script());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async { backend::reset_navicat() })
                .await;
            state
                .update(cx, |state, cx| {
                    state.set_script_result(result);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn render_titlebar(
        &self,
        window: &Window,
        state: &AppState,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let translations = state.t();
        let colors = state.theme_colors();
        let maximize_label = if window.is_maximized() { "❐" } else { "□" };

        div()
            .id("window-titlebar")
            .flex()
            .items_center()
            .w_full()
            .h(px(40.))
            .flex_shrink_0()
            .bg(components::color(colors.bg.canvas))
            .border_b_1()
            .border_color(components::color(colors.border.subtle))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pl_4()
                    .pr_6()
                    .h_full()
                    .flex_shrink_0()
                    .window_control_area(WindowControlArea::Drag)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(22.))
                            .h(px(22.))
                            .rounded_md()
                            .bg(components::color(colors.brand.fill))
                            .text_color(components::color(colors.fg.on_accent))
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .child("R"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(components::color(colors.fg.default))
                            .child(translations.app_title),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .window_control_area(WindowControlArea::Drag),
            )
            .child(
                components::window_control("—", colors, false)
                    .id("window-minimize")
                    .on_click(cx.listener(Self::minimize_window_clicked)),
            )
            .child(
                components::window_control(maximize_label, colors, false)
                    .id("window-maximize")
                    .on_click(cx.listener(Self::maximize_window_clicked)),
            )
            .child(
                components::window_control("×", colors, true)
                    .id("window-close")
                    .on_click(cx.listener(Self::close_window_clicked)),
            )
    }

    fn render_sidebar(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let navigation: [(View, &str, &str); 5] = [
            (View::Service, ICON_SERVICE, translations.tab_service),
            (View::Tools, ICON_TOOLS, translations.tab_tools),
            (View::Cleanup, ICON_CLEANUP, translations.tab_cleanup),
            (View::Scripts, ICON_SCRIPTS, translations.tab_scripts),
            (View::Settings, ICON_SETTINGS, translations.tab_settings),
        ];

        div()
            .flex()
            .flex_col()
            .w(px(220.))
            .h_full()
            .flex_shrink_0()
            .px_3()
            .py_4()
            .gap_1()
            .bg(components::color(colors.bg.sidebar))
            .border_r_1()
            .border_color(components::color(colors.border.subtle))
            .child(
                components::section_label(translations.nav_section, colors)
                    .px_3()
                    .pb_2(),
            )
            .children(navigation.into_iter().map(|(view, icon, label)| {
                let selected = state.current_view() == view;
                let id = match view {
                    View::Service => "sidebar-service",
                    View::Tools => "sidebar-tools",
                    View::Cleanup => "sidebar-cleanup",
                    View::Scripts => "sidebar-scripts",
                    View::Settings => "sidebar-settings",
                };
                components::sidebar_item(icon, label.trim(), colors, selected)
                    .id(id)
                    .on_click(cx.listener(move |this, event, window, cx| {
                        this.select_view(view, event, window, cx)
                    }))
            }))
    }

    fn render_content_header(&self, state: &AppState) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let (title, description) =
            if state.current_view() == View::Tools && state.tool_detail_active() {
                (translations.sysinfo_title, translations.tool_sysinfo_desc)
            } else {
                match state.current_view() {
                    View::Service => (translations.svc_header, translations.page_service_desc),
                    View::Tools => (translations.tools_header, translations.page_tools_desc),
                    View::Cleanup => (
                        translations.tab_cleanup.trim(),
                        translations.page_cleanup_desc,
                    ),
                    View::Scripts => (translations.scripts_header, translations.page_scripts_desc),
                    View::Settings => (
                        translations.settings_header,
                        translations.page_settings_desc,
                    ),
                }
            };
        let chip = match state.operation_state() {
            OperationState::Idle => None,
            OperationState::Refreshing | OperationState::LoadingServices => {
                Some((translations.status_refreshing, colors.brand.primary))
            }
            OperationState::Starting => Some((translations.status_starting, colors.warning.text)),
            OperationState::Stopping => Some((translations.status_stopping, colors.warning.text)),
            OperationState::LoadingSystemInfo => {
                Some((translations.sysinfo_fetching, colors.brand.primary))
            }
            OperationState::RunningScript => Some((translations.svc_pending, colors.warning.text)),
            OperationState::ScanningCleanup | OperationState::CleaningCleanup => {
                Some((translations.svc_pending, colors.warning.text))
            }
            OperationState::Error => Some((translations.err_failed, colors.danger.text)),
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .h(px(72.))
            .px_6()
            .flex_shrink_0()
            .border_b_1()
            .border_color(components::color(colors.border.subtle))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(components::color(colors.fg.default))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(components::color(colors.fg.muted))
                            .child(description),
                    ),
            )
            .children(chip.map(|(label, accent)| {
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .h(px(24.))
                    .px_2()
                    .rounded_full()
                    .bg(components::color(colors.bg.surface))
                    .border_1()
                    .border_color(components::color(colors.border.subtle))
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(components::color(accent))
                    .child(
                        div()
                            .w(px(6.))
                            .h(px(6.))
                            .rounded_full()
                            .bg(components::color(accent)),
                    )
                    .child(label)
            }))
    }

    /// Custom scrollbar: GPUI 0.2.2 does not paint scrollbars, so the track and
    /// thumb are drawn here and driven by a `ScrollHandle`.
    fn render_scrollbar(
        &self,
        handle: &ScrollHandle,
        colors: &ThemeColors,
        cx: &Context<Self>,
        id: &'static str,
        thumb_id: &'static str,
    ) -> gpui::Stateful<Div> {
        let viewport = f32::from(handle.bounds().size.height);
        let max_offset = f32::from(handle.max_offset().height);
        let track = div().relative().flex_shrink_0().w(px(10.)).h_full().id(id);

        if max_offset <= 0.0 || viewport <= 0.0 {
            return track;
        }

        let ratio = viewport / (viewport + max_offset);
        let thumb_height = (viewport * ratio).max(24.0);
        let scrolled = f32::from(-handle.offset().y).clamp(0.0, max_offset);
        let travel = (viewport - thumb_height).max(1.0);
        let thumb_top = travel * (scrolled / max_offset);

        track.child(
            div()
                .absolute()
                .top(px(thumb_top))
                .right(px(2.))
                .w(px(6.))
                .h(px(thumb_height))
                .rounded_full()
                .bg(components::color(colors.fg.subtle))
                .cursor_pointer()
                .hover(move |style| style.bg(components::color(colors.fg.muted)))
                .id(thumb_id)
                .on_drag_move::<ScrollbarDrag>(cx.listener(Self::on_scrollbar_drag))
                .on_drag(
                    ScrollbarDrag {
                        handle: handle.clone(),
                    },
                    |_, _, _, cx| cx.new(|_| ScrollbarDragPreview),
                ),
        )
    }

    fn on_scrollbar_drag(
        &mut self,
        event: &DragMoveEvent<ScrollbarDrag>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let handle = event.drag(cx).handle.clone();
        let viewport = f32::from(handle.bounds().size.height);
        let max_offset = f32::from(handle.max_offset().height);
        if max_offset <= 0.0 || viewport <= 0.0 {
            return;
        }
        let track_top = f32::from(handle.bounds().origin.y);
        let track_height = viewport;
        let thumb_height = (viewport * (viewport / (viewport + max_offset))).max(24.0);
        let travel = (track_height - thumb_height).max(1.0);
        let pointer = f32::from(event.event.position.y);
        let fraction = ((pointer - track_top - thumb_height / 2.0) / travel).clamp(0.0, 1.0);
        handle.set_offset(point(px(0.), px(-(max_offset * fraction))));
        cx.notify();
    }

    /// Keeps custom scrollbars in sync while the user scrolls with the wheel.
    fn on_scrolled(
        &mut self,
        _event: &gpui::ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.notify();
    }

    fn render_service_page(&self, window: &Window, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let refreshing = state.operation_state() == OperationState::Refreshing;

        let refresh_button: AnyElement = if refreshing {
            components::button(
                translations.status_refreshing,
                colors,
                components::ButtonVariant::Ghost,
            )
            .id("service-refresh")
            .on_click(cx.listener(Self::refresh_clicked))
            .with_animation(
                "service-refresh-pulse",
                Animation::new(Duration::from_millis(760)).repeat(),
                |element, delta| element.opacity(0.45 + 0.55 * delta),
            )
            .into_any_element()
        } else {
            components::button(
                translations.hint_refresh,
                colors,
                components::ButtonVariant::Ghost,
            )
            .id("service-refresh")
            .on_click(cx.listener(Self::refresh_clicked))
            .into_any_element()
        };

        let progress = refreshing.then(|| {
            components::progress_bar(colors)
                .absolute()
                .bottom_0()
                .left_0()
                .w_full()
        });
        let toolbar = div()
            .relative()
            .pb_2()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        components::button(
                            translations.hint_add,
                            colors,
                            components::ButtonVariant::Primary,
                        )
                        .id("service-add")
                        .debug_selector(|| "service-add".to_string())
                        .on_click(cx.listener(Self::add_clicked)),
                    )
                    .child(refresh_button),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        components::button(
                            translations.hint_start_all,
                            colors,
                            components::ButtonVariant::Secondary,
                        )
                        .id("service-start-all")
                        .on_click(cx.listener(Self::start_all_clicked)),
                    )
                    .child(
                        components::button(
                            translations.hint_stop_all,
                            colors,
                            components::ButtonVariant::Secondary,
                        )
                        .id("service-stop-all")
                        .on_click(cx.listener(Self::stop_all_clicked)),
                    ),
            )
            .children(progress);

        let search_focused = self.service_search_focus.is_focused(window);
        let search_input = {
            let input_entity = cx.entity();
            let input_focus = self.service_search_focus.clone();
            canvas(
                |_, _, _| (),
                move |bounds, _, window, cx| {
                    window.handle_input(
                        &input_focus,
                        ElementInputHandler::new(bounds, input_entity.clone()),
                        cx,
                    );
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        };
        let search_field = components::search_field(
            state.service_search(),
            state.service_search_marked(),
            translations.dialog_search,
            colors,
            search_focused,
        )
        .relative()
        .track_focus(&self.service_search_focus)
        .cursor_text()
        .focus(|style| style.border_color(components::color(colors.brand.primary)))
        .id("service-search")
        .on_click(cx.listener(Self::focus_service_search))
        .child(search_input);

        let filter_row = div()
            .flex()
            .items_center()
            .gap_3()
            .w_full()
            .child(div().flex_1().min_w(px(0.)).child(search_field))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        segment(
                            translations.filter_all,
                            state.service_filter() == ServiceFilter::All,
                            colors,
                            "filter-all",
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_state(cx, |state| {
                                state.set_service_filter(ServiceFilter::All)
                            })
                        })),
                    )
                    .child(
                        segment(
                            translations.status_running,
                            state.service_filter() == ServiceFilter::Running,
                            colors,
                            "filter-running",
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_state(cx, |state| {
                                state.set_service_filter(ServiceFilter::Running)
                            })
                        })),
                    )
                    .child(
                        segment(
                            translations.status_stopped,
                            state.service_filter() == ServiceFilter::Stopped,
                            colors,
                            "filter-stopped",
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_state(cx, |state| {
                                state.set_service_filter(ServiceFilter::Stopped)
                            })
                        })),
                    )
                    .child(
                        segment(
                            translations.svc_pending,
                            state.service_filter() == ServiceFilter::Pending,
                            colors,
                            "filter-pending",
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_state(cx, |state| {
                                state.set_service_filter(ServiceFilter::Pending)
                            })
                        })),
                    ),
            );

        let header = div()
            .flex()
            .items_center()
            .w_full()
            .h(px(36.))
            .px_3()
            .bg(components::color(colors.bg.muted))
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(components::color(colors.fg.muted))
            .child(
                div()
                    .w(px(SERVICE_NAME_COLUMN_WIDTH))
                    .child(translations.col_name),
            )
            .child(
                div()
                    .w(px(SERVICE_DISPLAY_COLUMN_WIDTH))
                    .child(translations.col_display),
            )
            .child(
                div()
                    .w(px(SERVICE_STATUS_COLUMN_WIDTH))
                    .child(translations.col_status),
            )
            .child(div().flex_1().child(translations.col_message))
            .child(
                div()
                    .w(px(SERVICE_ACTIONS_COLUMN_WIDTH))
                    .child(translations.col_actions),
            );

        let visible = state.filtered_service_indices();
        let rows = visible.iter().map(|&index| {
            let service = &state.managed_services()[index];
            let status = state.service_status(&service.name);
            let message = state.service_message(&service.name).unwrap_or("");
            let service_name = service.name.clone();
            let start_name = service_name.clone();
            let stop_name = service_name.clone();
            let detail_name = service_name.clone();
            let delete_name = service_name;
            let actions =
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .w(px(SERVICE_ACTIONS_COLUMN_WIDTH))
                    .child(
                        components::button(
                            translations.hint_start,
                            colors,
                            components::ButtonVariant::Ghost,
                        )
                        .h(px(28.))
                        .px_2()
                        .text_xs()
                        .id(("service-start", index))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.start_service(start_name.clone(), cx)
                        })),
                    )
                    .child(
                        components::button(
                            translations.hint_stop,
                            colors,
                            components::ButtonVariant::Ghost,
                        )
                        .h(px(28.))
                        .px_2()
                        .text_xs()
                        .id(("service-stop", index))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.stop_service(stop_name.clone(), cx)
                        })),
                    )
                    .child(
                        components::button(
                            translations.hint_detail,
                            colors,
                            components::ButtonVariant::Ghost,
                        )
                        .h(px(28.))
                        .px_2()
                        .text_xs()
                        .id(("service-detail", index))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_service_detail(detail_name.clone(), cx)
                        })),
                    )
                    .child(
                        components::button(
                            translations.hint_delete,
                            colors,
                            components::ButtonVariant::Danger,
                        )
                        .h(px(28.))
                        .px_2()
                        .text_xs()
                        .id(("service-delete", index))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.pending_delete = Some(delete_name.clone());
                            cx.notify();
                        })),
                    );
            div()
                .flex()
                .items_center()
                .w_full()
                .min_h(px(52.))
                .px_3()
                .border_b_1()
                .border_color(components::color(colors.border.subtle))
                .bg(components::color(colors.bg.surface))
                .text_sm()
                .text_color(components::color(colors.fg.default))
                .hover(move |style| style.bg(components::color(colors.bg.surface_hover)))
                .id(("service-row", index))
                .child(
                    div()
                        .w(px(SERVICE_NAME_COLUMN_WIDTH))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(service.name.clone()),
                )
                .child(
                    div()
                        .w(px(SERVICE_DISPLAY_COLUMN_WIDTH))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_color(components::color(colors.fg.muted))
                        .child(service.display_name.clone()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .w(px(SERVICE_STATUS_COLUMN_WIDTH))
                        .child(components::badge(
                            status_label(status, translations),
                            status_colors(status, colors),
                        )),
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_color(components::color(if message.is_empty() {
                            colors.fg.subtle
                        } else {
                            colors.danger.text
                        }))
                        .child(if message.is_empty() {
                            "—".to_string()
                        } else {
                            message.to_string()
                        }),
                )
                .child(actions)
        });

        let body = if state.managed_services().is_empty() {
            components::card(colors)
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .id("service-empty")
                .child(components::empty_state(
                    ICON_SERVICE,
                    translations.svc_header,
                    translations.svc_empty,
                    colors,
                ))
        } else if visible.is_empty() {
            components::card(colors)
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .id("service-no-match")
                .child(components::empty_state(
                    ICON_SERVICE,
                    translations.svc_no_match,
                    translations.dialog_search,
                    colors,
                ))
        } else {
            components::card(colors)
                .flex()
                .flex_col()
                .w_full()
                .flex_1()
                .overflow_hidden()
                .id("service-table")
                .child(header)
                .child(
                    div()
                        .flex()
                        .w_full()
                        .flex_1()
                        .overflow_hidden()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w(px(0.))
                                .h_full()
                                .id("service-list")
                                .overflow_y_scroll()
                                .track_scroll(&self.service_scroll)
                                .on_scroll_wheel(cx.listener(Self::on_scrolled))
                                .children(rows),
                        )
                        .child(self.render_scrollbar(
                            &self.service_scroll,
                            colors,
                            cx,
                            "service-scrollbar",
                            "service-thumb",
                        )),
                )
        };

        let notice = state.refresh_notice().map(|text| {
            div()
                .absolute()
                .bottom_4()
                .right_6()
                .px_3()
                .py_2()
                .rounded_lg()
                .bg(components::color(colors.bg.elevated))
                .border_1()
                .border_color(components::color(colors.border.default))
                .shadow_lg()
                .text_sm()
                .text_color(components::color(colors.success.text))
                .child(text.to_string())
        });

        div()
            .relative()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .p_6()
            .child(toolbar)
            .child(filter_row)
            .child(body)
            .children(notice)
    }

    fn render_tools_page(&self, state: &AppState, cx: &Context<Self>) -> Div {
        if state.tool_detail_active() {
            return self.render_system_info(state, cx);
        }
        let translations = state.t();
        let colors = state.theme_colors();
        let rows = TOOLS
            .iter()
            .enumerate()
            .map(|(index, (name, description))| {
                let label = tool_text(translations, name);
                let description = tool_text(translations, description);
                let selected = state.tools_selected() == index;
                components::card(colors)
                    .flex()
                    .items_center()
                    .gap_3()
                    .p_4()
                    .cursor_pointer()
                    .border_color(components::color(if selected {
                        colors.brand.primary
                    } else {
                        colors.border.subtle
                    }))
                    .bg(components::color(if selected {
                        colors.brand.soft
                    } else {
                        colors.bg.surface
                    }))
                    .id(("tool-row", index))
                    .on_click(cx.listener(move |this, event, window, cx| {
                        this.select_tool(index, event, window, cx);
                        if index == 0 {
                            this.open_tool(cx);
                        }
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(40.))
                            .h(px(40.))
                            .rounded_lg()
                            .flex_shrink_0()
                            .bg(components::color(if selected {
                                colors.bg.surface
                            } else {
                                colors.bg.muted
                            }))
                            .text_lg()
                            .text_color(components::color(colors.brand.primary))
                            .child(ICON_TOOLS),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_base()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(components::color(colors.fg.default))
                                    .child(label),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(components::color(colors.fg.muted))
                                    .child(description),
                            ),
                    )
                    .child(
                        div()
                            .text_lg()
                            .text_color(components::color(colors.fg.subtle))
                            .child("›"),
                    )
            });
        div().flex().flex_col().size_full().p_6().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .w_full()
                .flex_1()
                .children(rows)
                .id("tool-list")
                .overflow_y_scroll(),
        )
    }

    fn render_system_info(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let content = if state.system_info_loading() {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(components::color(colors.fg.muted))
                .id("system-info-loading")
                .child(translations.sysinfo_fetching)
        } else if let Some(info) = state.system_info() {
            let system_rows = [
                (translations.sysinfo_os, info.os.as_str()),
                (translations.sysinfo_version, info.version.as_str()),
                (translations.sysinfo_build, info.build.as_str()),
                (translations.sysinfo_computer, info.computer.as_str()),
                (translations.sysinfo_user, info.user.as_str()),
            ];
            let hardware_rows = [
                (translations.sysinfo_cpu, info.cpu.as_str()),
                (translations.sysinfo_cores, info.cores.as_str()),
                (translations.sysinfo_ram, info.ram.as_str()),
            ];
            div()
                .flex()
                .w_full()
                .flex_1()
                .overflow_hidden()
                .id("system-info-scroll")
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap_4()
                        .flex_1()
                        .min_w(px(0.))
                        .h_full()
                        .id("system-info-groups")
                        .overflow_y_scroll()
                        .track_scroll(&self.sysinfo_scroll)
                        .on_scroll_wheel(cx.listener(Self::on_scrolled))
                        .child(info_group(
                            translations.sysinfo_group_system,
                            &system_rows,
                            colors,
                        ))
                        .child(info_group(
                            translations.sysinfo_group_hardware,
                            &hardware_rows,
                            colors,
                        )),
                )
                .child(self.render_scrollbar(
                    &self.sysinfo_scroll,
                    colors,
                    cx,
                    "sysinfo-scrollbar",
                    "sysinfo-thumb",
                ))
        } else {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(components::color(colors.danger.text))
                .id("system-info-error")
                .child(translations.err_failed)
        };
        div()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .p_6()
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        components::button(
                            translations.hint_refresh,
                            colors,
                            components::ButtonVariant::Secondary,
                        )
                        .id("sysinfo-refresh")
                        .on_click(cx.listener(Self::refresh_system_info_clicked)),
                    )
                    .child(
                        components::button(
                            translations.sysinfo_back,
                            colors,
                            components::ButtonVariant::Secondary,
                        )
                        .id("sysinfo-back")
                        .on_click(cx.listener(
                            |this, _event, _window, cx| {
                                this.update_state(cx, |state| state.close_tool_detail());
                            },
                        )),
                    ),
            )
            .child(content)
    }

    fn render_cleanup_page(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let cleanup = state.cleanup();

        let toolbar = div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        components::button(
                            translations.cleanup_scan,
                            colors,
                            components::ButtonVariant::Secondary,
                        )
                        .id("cleanup-scan")
                        .debug_selector(|| "cleanup-scan".to_string())
                        .on_click(cx.listener(Self::cleanup_scan_clicked)),
                    )
                    .child(
                        components::button(
                            translations.cleanup_clean,
                            colors,
                            components::ButtonVariant::Primary,
                        )
                        .id("cleanup-clean")
                        .on_click(cx.listener(Self::cleanup_clean_clicked)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        segment(
                            translations.cleanup_mode_recycle,
                            cleanup.delete_mode == DeleteMode::Recycle,
                            colors,
                            "cleanup-mode-recycle",
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_state(cx, |state| {
                                state.set_cleanup_delete_mode(DeleteMode::Recycle)
                            })
                        })),
                    )
                    .child(
                        segment(
                            translations.cleanup_mode_permanent,
                            cleanup.delete_mode == DeleteMode::Permanent,
                            colors,
                            "cleanup-mode-permanent",
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_state(cx, |state| {
                                state.set_cleanup_delete_mode(DeleteMode::Permanent)
                            })
                        })),
                    ),
            );

        let rows = cleanup.rows.iter().enumerate().map(|(index, row)| {
            let selected = row.selected;
            let checkbox = div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(16.))
                .h(px(16.))
                .rounded_sm()
                .border_1()
                .border_color(components::color(if selected {
                    colors.brand.fill
                } else {
                    colors.border.strong
                }))
                .bg(components::color(if selected {
                    colors.brand.fill
                } else {
                    colors.bg.canvas
                }))
                .text_xs()
                .text_color(components::color(colors.fg.on_accent))
                .child(if selected { "✓" } else { "" });
            let size = match row.scan {
                Some(scan) => format!(
                    "{}  ·  {}",
                    cleanup::format_bytes(scan.bytes),
                    translations
                        .dialog_count
                        .replace("{}", &scan.files.to_string())
                ),
                None => translations.cleanup_not_scanned.to_string(),
            };
            components::card(colors)
                .flex()
                .items_center()
                .gap_3()
                .p_3()
                .cursor_pointer()
                .id(("cleanup-row", index))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_state(cx, move |state| state.toggle_cleanup_row(index))
                }))
                .child(checkbox)
                .child(
                    div()
                        .flex_1()
                        .child(cleanup_category_label(row.category, translations)),
                )
                .children(
                    row.category
                        .requires_admin()
                        .then(|| components::badge(translations.cleanup_admin, &colors.info)),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(components::color(colors.fg.muted))
                        .child(size),
                )
        });

        let recycle_row = components::card(colors)
            .flex()
            .items_center()
            .gap_3()
            .p_3()
            .child(div().flex_1().child(translations.cleanup_recycle_bin))
            .child(
                div()
                    .text_sm()
                    .text_color(components::color(colors.fg.muted))
                    .child(match cleanup.recycle_bin {
                        Some((bytes, _items)) => cleanup::format_bytes(bytes),
                        None => translations.cleanup_not_scanned.to_string(),
                    }),
            )
            .child(
                components::button(
                    translations.cleanup_empty_bin,
                    colors,
                    components::ButtonVariant::Secondary,
                )
                .id("cleanup-empty-bin")
                .on_click(cx.listener(Self::cleanup_empty_bin_clicked)),
            );

        let message = cleanup
            .message
            .clone()
            .map(|text| components::alert(text, &colors.info));

        div()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .p_6()
            .child(toolbar)
            .children(message)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .w_full()
                    .flex_1()
                    .id("cleanup-list")
                    .overflow_y_scroll()
                    .children(rows)
                    .child(recycle_row),
            )
    }

    fn render_scripts_page(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let row = components::card(colors)
            .flex()
            .items_center()
            .gap_3()
            .w_full()
            .p_4()
            .cursor_pointer()
            .id("script-reset-navicat")
            .on_click(cx.listener(Self::script_clicked))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(40.))
                    .h(px(40.))
                    .rounded_lg()
                    .flex_shrink_0()
                    .bg(components::color(colors.warning.soft))
                    .text_lg()
                    .text_color(components::color(colors.warning.text))
                    .child(ICON_SCRIPTS),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .flex_1()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(components::color(colors.fg.default))
                            .child(translations.script_reset_navicat),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(components::color(colors.fg.muted))
                            .child(translations.script_reset_navicat_desc),
                    ),
            )
            .child(
                components::button(
                    translations.hint_confirm,
                    colors,
                    components::ButtonVariant::Primary,
                )
                .id("script-run")
                .on_click(cx.listener(Self::script_clicked)),
            );
        let mut page = div().flex().flex_col().gap_4().size_full().p_6().child(row);
        if let Some(result) = state.script_result() {
            page = page.child(components::alert(result.to_string(), &colors.info));
        }
        page
    }

    fn render_settings_page(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let language = state.language();
        let theme = state.theme();

        let language_row = components::card(colors)
            .flex()
            .items_center()
            .justify_between()
            .p_4()
            .id("setting-row-0")
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(components::color(colors.fg.default))
                    .child(translations.setting_language),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        segment(
                            Language::Chinese.name(),
                            language == Language::Chinese,
                            colors,
                            "setting-language-zh",
                        )
                        .on_click(cx.listener(Self::set_language_zh)),
                    )
                    .child(
                        segment(
                            Language::English.name(),
                            language == Language::English,
                            colors,
                            "setting-language-en",
                        )
                        .on_click(cx.listener(Self::set_language_en)),
                    ),
            );

        let theme_row = components::card(colors)
            .flex()
            .items_center()
            .justify_between()
            .p_4()
            .id("setting-row-1")
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(components::color(colors.fg.default))
                    .child(translations.setting_theme),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        segment(
                            translations.theme_dark,
                            theme == Theme::Dark,
                            colors,
                            "setting-theme-dark",
                        )
                        .on_click(cx.listener(Self::set_theme_dark)),
                    )
                    .child(
                        segment(
                            translations.theme_light,
                            theme == Theme::Light,
                            colors,
                            "setting-theme-light",
                        )
                        .on_click(cx.listener(Self::set_theme_light)),
                    ),
            );

        div()
            .flex()
            .flex_col()
            .gap_3()
            .size_full()
            .p_6()
            .child(language_row)
            .child(theme_row)
    }

    fn render_add_dialog(&self, window: &Window, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let list = if state.add_dialog_loading() {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(components::color(colors.fg.muted))
                .child(translations.status_refreshing)
                .id("dialog-loading")
        } else if state.add_dialog_filtered().is_empty() {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(components::empty_state(
                    ICON_SERVICE,
                    translations.dialog_empty,
                    translations.dialog_search,
                    colors,
                ))
                .id("dialog-empty")
        } else {
            let rows =
                state
                    .add_dialog_filtered()
                    .iter()
                    .enumerate()
                    .map(|(index, &real_index)| {
                        let service = &state.add_dialog_services()[real_index];
                        let selected = state.add_dialog_selected() == index;
                        div()
                            .flex()
                            .items_center()
                            .w_full()
                            .p_3()
                            .rounded_lg()
                            .border_1()
                            .border_color(components::color(if selected {
                                colors.brand.primary
                            } else {
                                colors.border.subtle
                            }))
                            .bg(components::color(if selected {
                                colors.brand.soft
                            } else {
                                colors.bg.surface
                            }))
                            .text_sm()
                            .text_color(components::color(colors.fg.default))
                            .cursor_pointer()
                            .hover(move |style| {
                                style.bg(components::color(colors.bg.surface_hover))
                            })
                            .id(("dialog-service", index))
                            .on_click(cx.listener(move |this, event, window, cx| {
                                this.select_dialog_service(index, event, window, cx);
                            }))
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .child(format!("{} — {}", service.name, service.display_name)),
                            )
                    });
            div()
                .flex()
                .flex_1()
                .w_full()
                .overflow_hidden()
                .id("dialog-scroll-area")
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .flex_1()
                        .min_w(px(0.))
                        .h_full()
                        .id("dialog-service-list")
                        .overflow_y_scroll()
                        .track_scroll(&self.dialog_scroll)
                        .on_scroll_wheel(cx.listener(Self::on_scrolled))
                        .children(rows),
                )
                .child(self.render_scrollbar(
                    &self.dialog_scroll,
                    colors,
                    cx,
                    "dialog-scrollbar",
                    "dialog-thumb",
                ))
        };
        let error = state
            .add_dialog_error()
            .map(|error| components::alert(error.to_string(), &colors.danger));
        let count = translations
            .dialog_count
            .replace("{}", &state.add_dialog_filtered().len().to_string());

        // The search field is the only text input. A transparent canvas overlay
        // registers the platform input handler each paint, which is what enables
        // IME composition (e.g. Chinese) on Windows.
        let focused = self.search_focus.is_focused(window);
        let input_entity = cx.entity();
        let input_focus = self.search_focus.clone();
        let search_input = canvas(
            |_, _, _| (),
            move |bounds, _, window, cx| {
                window.handle_input(
                    &input_focus,
                    ElementInputHandler::new(bounds, input_entity.clone()),
                    cx,
                );
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let search = components::search_field(
            state.add_dialog_search(),
            state.add_dialog_marked(),
            translations.dialog_search,
            colors,
            focused,
        )
        .relative()
        .track_focus(&self.search_focus)
        .cursor_text()
        .focus(|style| style.border_color(components::color(colors.brand.primary)))
        .id("dialog-search")
        .on_click(cx.listener(Self::focus_search))
        .child(search_input);

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x00000099))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .w(px(560.))
                    .h(px(520.))
                    .p_5()
                    .rounded_xl()
                    .shadow_2xl()
                    .bg(components::color(colors.bg.elevated))
                    .border_1()
                    .border_color(components::color(colors.border.default))
                    .text_color(components::color(colors.fg.default))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(translations.dialog_add_title),
                            )
                            .child(
                                components::button("✕", colors, components::ButtonVariant::Ghost)
                                    .w(px(28.))
                                    .h(px(28.))
                                    .px_0()
                                    .id("dialog-close")
                                    .on_click(cx.listener(Self::close_dialog_clicked)),
                            ),
                    )
                    .child(search)
                    .children(error)
                    .child(list)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(components::color(colors.fg.subtle))
                                    .child(count),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        components::button(
                                            translations.dialog_hint_cancel,
                                            colors,
                                            components::ButtonVariant::Secondary,
                                        )
                                        .id("dialog-cancel")
                                        .on_click(cx.listener(Self::cancel_dialog_clicked)),
                                    )
                                    .child(
                                        components::button(
                                            translations.dialog_hint_add,
                                            colors,
                                            components::ButtonVariant::Primary,
                                        )
                                        .id("dialog-confirm")
                                        .on_click(cx.listener(Self::confirm_dialog_clicked)),
                                    ),
                            ),
                    ),
            )
    }

    fn add_clicked(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.begin_add_dialog(cx);
        window.focus(&self.search_focus);
    }

    fn start_all_clicked(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.start_all(cx);
    }

    fn stop_all_clicked(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.stop_all(cx);
    }

    fn refresh_clicked(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_services(cx);
    }

    fn refresh_system_info_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.request_system_info_refresh());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async { backend::fetch_system_info() })
                .await;
            state
                .update(cx, |state, cx| {
                    state.set_system_info(result);
                    cx.notify();
                })
                .ok();
            Timer::after(Duration::from_millis(100)).await;
            state.update(cx, |_state, cx| cx.notify()).ok();
        })
        .detach();
    }

    fn script_clicked(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.run_script(cx);
    }

    fn focus_search(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.search_focus);
        cx.notify();
    }

    fn focus_service_search(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.service_search_focus);
        cx.notify();
    }

    fn open_service_detail(&mut self, name: String, cx: &mut Context<Self>) {
        let lookup = name.clone();
        self.update_state(cx, move |state| state.begin_service_detail(&name));
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { backend::service_details(&lookup) })
                .await;
            state
                .update(cx, |state, cx| {
                    state.set_service_detail(result);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn close_service_detail_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.close_service_detail());
    }

    fn set_start_type(&mut self, name: String, start_type: StartType, cx: &mut Context<Self>) {
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { backend::set_service_start_type(&name, start_type) })
                .await;
            state
                .update(cx, |state, cx| {
                    match result {
                        Ok(()) => state.apply_start_type(start_type),
                        Err(error) => state.set_service_detail_error(&error),
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn confirm_delete_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(name) = self.pending_delete.take() {
            self.update_state(cx, move |state| state.remove_service(&name));
        }
    }

    fn cancel_delete_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_delete = None;
        cx.notify();
    }

    fn cleanup_scan_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.begin_cleanup_scan());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let scans = cx
                .background_spawn(async {
                    CleanupCategory::all()
                        .iter()
                        .map(|&category| (category, cleanup::scan(category)))
                        .collect::<Vec<_>>()
                })
                .await;
            let recycle_bin = cx
                .background_spawn(async { cleanup::recycle_bin_usage() })
                .await;
            state
                .update(cx, |state, cx| {
                    state.apply_cleanup_scan(scans, recycle_bin);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn cleanup_clean_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_clean = true;
        cx.notify();
    }

    fn confirm_clean_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.pending_clean {
            return;
        }
        self.pending_clean = false;
        let categories = self.state.read(cx).selected_cleanup_categories();
        let mode = self.state.read(cx).cleanup().delete_mode;
        self.update_state(cx, |state| state.begin_cleanup());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let rescan = categories.clone();
            let report = cx
                .background_spawn(async move { cleanup::clean(&categories, mode) })
                .await;
            let scans = cx
                .background_spawn(async move {
                    rescan
                        .iter()
                        .map(|&category| (category, cleanup::scan(category)))
                        .collect::<Vec<_>>()
                })
                .await;
            let recycle_bin = cx
                .background_spawn(async { cleanup::recycle_bin_usage() })
                .await;
            state
                .update(cx, |state, cx| {
                    let message = state
                        .t()
                        .cleanup_done
                        .replace("{}", &cleanup::format_bytes(report.freed_bytes))
                        .replace("{}", &report.skipped_files.to_string());
                    state.apply_cleanup_result(scans, recycle_bin, message);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn cancel_clean_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_clean = false;
        cx.notify();
    }

    fn cleanup_empty_bin_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_empty_bin = true;
        cx.notify();
    }

    fn confirm_empty_bin_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.pending_empty_bin {
            return;
        }
        self.pending_empty_bin = false;
        self.update_state(cx, |state| state.begin_cleanup());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let emptied = cx
                .background_spawn(async { cleanup::empty_recycle_bin() })
                .await;
            let recycle_bin = cx
                .background_spawn(async { cleanup::recycle_bin_usage() })
                .await;
            state
                .update(cx, |state, cx| {
                    state.apply_cleanup_scan(Vec::new(), recycle_bin);
                    if emptied {
                        let message = state.t().cleanup_bin_done.to_string();
                        state.set_cleanup_message(message);
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn cancel_empty_bin_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_empty_bin = false;
        cx.notify();
    }

    fn cancel_dialog_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.close_add_dialog());
    }

    fn close_dialog_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.close_add_dialog());
    }

    fn confirm_dialog_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.state.read(cx).add_dialog_loading() {
            self.update_state(cx, |state| {
                state.confirm_add_service();
            });
        }
    }

    fn render_service_detail(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let name = state.service_detail_name().unwrap_or_default().to_string();

        let body: AnyElement = if state.service_detail_loading() {
            div()
                .flex()
                .items_center()
                .justify_center()
                .py_6()
                .w_full()
                .text_color(components::color(colors.fg.muted))
                .child(translations.status_refreshing)
                .into_any_element()
        } else if let Some(error) = state.service_detail_error() {
            div()
                .flex()
                .items_center()
                .justify_center()
                .py_6()
                .w_full()
                .text_color(components::color(colors.danger.text))
                .child(error.to_string())
                .into_any_element()
        } else if let Some(details) = state.service_detail() {
            let status = ServiceStatus::from_backend(&details.status);
            let start_type = details.start_type;
            let segment_name = details.name.clone();
            let depends_on = join_list(&details.depends_on, translations.detail_none);
            let dependents = join_list(&details.dependents, translations.detail_none);
            div()
                .flex()
                .flex_col()
                .gap_2()
                .w_full()
                .child(detail_row(
                    translations.detail_status,
                    status_label(status, translations).to_string(),
                    colors,
                ))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .w_full()
                        .child(
                            div()
                                .w(px(96.))
                                .text_sm()
                                .text_color(components::color(colors.fg.muted))
                                .child(translations.detail_start_type),
                        )
                        .child({
                            let automatic = segment_name.clone();
                            segment(
                                translations.start_type_auto,
                                start_type == StartType::Automatic,
                                colors,
                                "detail-start-auto",
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.set_start_type(automatic.clone(), StartType::Automatic, cx)
                                },
                            ))
                        })
                        .child({
                            let manual = segment_name.clone();
                            segment(
                                translations.start_type_manual,
                                start_type == StartType::Manual,
                                colors,
                                "detail-start-manual",
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.set_start_type(manual.clone(), StartType::Manual, cx)
                                },
                            ))
                        })
                        .child({
                            let disabled = segment_name;
                            segment(
                                translations.start_type_disabled,
                                start_type == StartType::Disabled,
                                colors,
                                "detail-start-disabled",
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.set_start_type(disabled.clone(), StartType::Disabled, cx)
                                },
                            ))
                        }),
                )
                .child(detail_row(
                    translations.detail_binary,
                    details.binary_path.clone(),
                    colors,
                ))
                .child(detail_row(
                    translations.detail_account,
                    details.account.clone(),
                    colors,
                ))
                .child(detail_row(
                    translations.detail_pid,
                    details.process_id.to_string(),
                    colors,
                ))
                .child(detail_row(
                    translations.detail_description,
                    details.description.clone(),
                    colors,
                ))
                .child(detail_row(
                    translations.detail_depends_on,
                    depends_on,
                    colors,
                ))
                .child(detail_row(
                    translations.detail_dependents,
                    dependents,
                    colors,
                ))
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x00000099))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .w(px(560.))
                    .max_h(px(560.))
                    .p_5()
                    .rounded_xl()
                    .shadow_2xl()
                    .bg(components::color(colors.bg.elevated))
                    .border_1()
                    .border_color(components::color(colors.border.default))
                    .text_color(components::color(colors.fg.default))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(translations.detail_title),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(components::color(colors.fg.muted))
                                            .child(name),
                                    ),
                            )
                            .child(
                                components::button("✕", colors, components::ButtonVariant::Ghost)
                                    .w(px(28.))
                                    .h(px(28.))
                                    .px_0()
                                    .id("detail-close")
                                    .debug_selector(|| "detail-close".to_string())
                                    .on_click(cx.listener(Self::close_service_detail_clicked)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .w_full()
                            .flex_1()
                            .id("detail-body")
                            .overflow_y_scroll()
                            .child(body),
                    ),
            )
    }

    fn render_confirm_dialog(
        &self,
        state: &AppState,
        cx: &Context<Self>,
        dialog: ConfirmDialog,
    ) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let ok_variant = if dialog.danger {
            components::ButtonVariant::Danger
        } else {
            components::ButtonVariant::Primary
        };
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x00000099))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .w(px(420.))
                    .p_5()
                    .rounded_xl()
                    .shadow_2xl()
                    .bg(components::color(colors.bg.elevated))
                    .border_1()
                    .border_color(components::color(colors.border.default))
                    .text_color(components::color(colors.fg.default))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(dialog.title),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(components::color(colors.fg.muted))
                            .child(dialog.message),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                components::button(
                                    translations.dialog_hint_cancel,
                                    colors,
                                    components::ButtonVariant::Secondary,
                                )
                                .id(dialog.cancel_id)
                                .on_click(cx.listener(dialog.on_cancel)),
                            )
                            .child(
                                components::button(dialog.ok_label, colors, ok_variant)
                                    .id(dialog.ok_id)
                                    .on_click(cx.listener(dialog.on_ok)),
                            ),
                    ),
            )
    }

    fn render_confirm_delete(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let name = self.pending_delete.clone().unwrap_or_default();
        let message = translations.confirm_delete_message.replace("{}", &name);
        self.render_confirm_dialog(
            state,
            cx,
            ConfirmDialog {
                title: translations.confirm_delete_title,
                message,
                ok_label: translations.hint_delete,
                ok_id: "confirm-delete-ok",
                cancel_id: "confirm-delete-cancel",
                danger: true,
                on_ok: Self::confirm_delete_clicked,
                on_cancel: Self::cancel_delete_clicked,
            },
        )
    }

    fn render_confirm_clean(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let totals = state.selected_cleanup_totals();
        let mode = match state.cleanup().delete_mode {
            DeleteMode::Recycle => translations.cleanup_mode_recycle,
            DeleteMode::Permanent => translations.cleanup_mode_permanent,
        };
        let message = translations
            .cleanup_confirm_message
            .replace("{}", &cleanup::format_bytes(totals.bytes))
            .replace("{}", &totals.files.to_string())
            .replace("{}", mode);
        self.render_confirm_dialog(
            state,
            cx,
            ConfirmDialog {
                title: translations.cleanup_confirm_title,
                message,
                ok_label: translations.cleanup_clean,
                ok_id: "confirm-clean-ok",
                cancel_id: "confirm-clean-cancel",
                danger: false,
                on_ok: Self::confirm_clean_clicked,
                on_cancel: Self::cancel_clean_clicked,
            },
        )
    }

    fn render_confirm_empty_bin(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let (bytes, items) = state.cleanup().recycle_bin.unwrap_or((0, 0));
        let message = translations
            .cleanup_bin_confirm_message
            .replace("{}", &items.to_string())
            .replace("{}", &cleanup::format_bytes(bytes));
        self.render_confirm_dialog(
            state,
            cx,
            ConfirmDialog {
                title: translations.cleanup_bin_confirm_title,
                message,
                ok_label: translations.cleanup_empty_bin,
                ok_id: "confirm-empty-bin-ok",
                cancel_id: "confirm-empty-bin-cancel",
                danger: true,
                on_ok: Self::confirm_empty_bin_clicked,
                on_cancel: Self::cancel_empty_bin_clicked,
            },
        )
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let page = match state.current_view() {
            View::Service => self.render_service_page(window, state, cx),
            View::Tools => self.render_tools_page(state, cx),
            View::Cleanup => self.render_cleanup_page(state, cx),
            View::Scripts => self.render_scripts_page(state, cx),
            View::Settings => self.render_settings_page(state, cx),
        };
        let mut content = div()
            .flex()
            .flex_1()
            .w_full()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down))
            .child(self.render_sidebar(state, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .child(self.render_content_header(state))
                    .child(div().flex_1().w_full().child(page)),
            );
        if state.show_add_dialog() {
            content = content.child(self.render_add_dialog(window, state, cx));
        }
        if self.pending_delete.is_some() {
            content = content.child(self.render_confirm_delete(state, cx));
        }
        if self.pending_clean {
            content = content.child(self.render_confirm_clean(state, cx));
        }
        if self.pending_empty_bin {
            content = content.child(self.render_confirm_empty_bin(state, cx));
        }
        if state.service_detail_name().is_some() {
            content = content.child(self.render_service_detail(state, cx));
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(components::color(state.theme_colors().bg.canvas))
            .text_color(components::color(state.theme_colors().fg.default))
            .child(self.render_titlebar(window, state, cx))
            .child(content)
    }
}

impl EntityInputHandler for MainWindow {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let service = self.service_search_focus.is_focused(window);
        let state = self.state.read(cx);
        let text = if service {
            state.service_search_text_range(range_utf16.clone())
        } else {
            state.add_dialog_text_range(range_utf16.clone())
        };
        *adjusted_range = Some(range_utf16);
        Some(text)
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let state = self.state.read(cx);
        let cursor = if self.service_search_focus.is_focused(window) {
            state.service_search_text_utf16_len()
        } else {
            state.add_dialog_text_utf16_len()
        };
        Some(UTF16Selection {
            range: cursor..cursor,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let state = self.state.read(cx);
        if self.service_search_focus.is_focused(window) {
            state.service_search_marked_range()
        } else {
            state.add_dialog_marked_range()
        }
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let service = self.service_search_focus.is_focused(window);
        self.update_state(cx, move |state| {
            if service {
                state.service_search_unmark();
            } else {
                state.add_dialog_unmark();
            }
        });
    }

    fn replace_text_in_range(
        &mut self,
        replacement_range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = text.to_string();
        let service = self.service_search_focus.is_focused(window);
        self.update_state(cx, move |state| {
            if service {
                match replacement_range {
                    Some(range) => state.service_search_replace_range(range, &text),
                    None => state.service_search_commit_text(&text),
                }
            } else {
                match replacement_range {
                    Some(range) => state.add_dialog_replace_range(range, &text),
                    None => state.add_dialog_commit_text(&text),
                }
            }
        });
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = new_text.to_string();
        let service = self.service_search_focus.is_focused(window);
        self.update_state(cx, move |state| {
            if service {
                if let Some(range) = range_utf16 {
                    state.service_search_replace_range(range, "");
                }
                state.service_search_set_marked(&text);
            } else {
                if let Some(range) = range_utf16 {
                    state.add_dialog_replace_range(range, "");
                }
                state.add_dialog_set_marked(&text);
            }
        });
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(element_bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

#[cfg(target_os = "windows")]
fn toggle_window_zoom(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsZoomed, ShowWindowAsync, SW_MAXIMIZE, SW_RESTORE,
    };

    let Ok(window_handle) = <Window as HasWindowHandle>::window_handle(window) else {
        window.zoom_window();
        return;
    };
    let RawWindowHandle::Win32(handle) = window_handle.as_raw() else {
        window.zoom_window();
        return;
    };
    let hwnd = handle.hwnd.get() as windows_sys::Win32::Foundation::HWND;
    let command = unsafe {
        if IsZoomed(hwnd) != 0 {
            SW_RESTORE
        } else {
            SW_MAXIMIZE
        }
    };
    unsafe {
        ShowWindowAsync(hwnd, command);
    }
}

#[cfg(not(target_os = "windows"))]
fn toggle_window_zoom(window: &Window) {
    window.zoom_window();
}

/// A titled group of label/value rows for the system information page.
fn info_group(title: &'static str, rows: &[(&str, &str)], colors: &ThemeColors) -> Div {
    components::card(colors)
        .flex()
        .flex_col()
        .gap_4()
        .flex_1()
        .p_4()
        .child(components::card_title(title, colors))
        .children(rows.iter().map(|(label, value)| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(components::color(colors.fg.subtle))
                        .child((*label).to_string()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(components::color(colors.fg.default))
                        .child((*value).to_string()),
                )
        }))
}

/// A two-state pill used by the settings rows.
fn segment(
    label: &str,
    active: bool,
    colors: &ThemeColors,
    id: &'static str,
) -> gpui::Stateful<Div> {
    let base = div()
        .flex()
        .items_center()
        .justify_center()
        .h(px(28.))
        .px_3()
        .rounded_md()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .id(id);
    let base = if active {
        base.bg(components::color(colors.brand.soft))
            .text_color(components::color(colors.brand.primary))
    } else {
        base.bg(components::color(colors.bg.canvas))
            .text_color(components::color(colors.fg.muted))
            .hover(move |style| style.bg(components::color(colors.bg.surface_hover)))
    };
    base.child(label.to_string())
}

fn status_label(status: ServiceStatus, translations: &crate::i18n::Translations) -> &'static str {
    match status {
        ServiceStatus::Running => translations.status_running,
        ServiceStatus::Stopped | ServiceStatus::Unknown => translations.status_stopped,
        ServiceStatus::Starting => translations.status_starting,
        ServiceStatus::Stopping => translations.status_stopping,
        ServiceStatus::Refreshing => translations.status_refreshing,
    }
}

fn status_colors(status: ServiceStatus, colors: &ThemeColors) -> &StatusColors {
    match status {
        ServiceStatus::Running => &colors.success,
        ServiceStatus::Stopped => &colors.danger,
        ServiceStatus::Starting | ServiceStatus::Stopping | ServiceStatus::Refreshing => {
            &colors.warning
        }
        ServiceStatus::Unknown => &colors.info,
    }
}

fn detail_row(label: &str, value: String, colors: &ThemeColors) -> Div {
    div()
        .flex()
        .items_start()
        .gap_2()
        .w_full()
        .child(
            div()
                .w(px(96.))
                .flex_shrink_0()
                .text_sm()
                .text_color(components::color(colors.fg.muted))
                .child(label.to_string()),
        )
        .child(
            div()
                .flex_1()
                .text_sm()
                .text_color(components::color(colors.fg.default))
                .child(value),
        )
}

fn join_list(items: &[String], empty: &str) -> String {
    if items.is_empty() {
        empty.to_string()
    } else {
        items.join(", ")
    }
}

fn cleanup_category_label(
    category: CleanupCategory,
    translations: &crate::i18n::Translations,
) -> &'static str {
    match category {
        CleanupCategory::UserTemp => translations.cleanup_cat_user_temp,
        CleanupCategory::WindowsTemp => translations.cleanup_cat_windows_temp,
        CleanupCategory::ThumbnailCache => translations.cleanup_cat_thumbnails,
        CleanupCategory::WindowsUpdate => translations.cleanup_cat_windows_update,
    }
}

fn tool_text(translations: &crate::i18n::Translations, key: &str) -> &'static str {
    match key {
        "tool_sysinfo" => translations.tool_sysinfo,
        "tool_sysinfo_desc" => translations.tool_sysinfo_desc,
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::{MainWindow, SERVICE_STATUS_COLUMN_WIDTH, TOOLS};
    use crate::app::{AppState, View};
    use gpui::{point, px, size, AppContext, Modifiers, MouseButton, TestAppContext};

    #[gpui::test]
    fn main_window_entity_smoke(cx: &mut TestAppContext) {
        let state = cx.new(|_| AppState::new());
        let window = cx.new(|cx| MainWindow::new(state.clone(), cx));
        let state_id = cx.read(|app| window.read(app).state.entity_id());
        assert_eq!(state_id, state.entity_id());
    }

    #[gpui::test]
    fn main_window_render_smoke(cx: &mut TestAppContext) {
        let state = cx.new(|_| AppState::new());
        let (view, visual_cx) = cx.add_window_view(|_, cx| MainWindow::new(state.clone(), cx));
        let _ = visual_cx.draw(point(px(0.), px(0.)), size(px(1100.), px(720.)), |_, _| {
            view.clone()
        });
    }

    #[gpui::test]
    fn main_window_renders_minimum_shell_size(cx: &mut TestAppContext) {
        let state = cx.new(|_| AppState::new());
        let (view, visual_cx) = cx.add_window_view(|_, cx| MainWindow::new(state.clone(), cx));
        let _ = visual_cx.draw(point(px(0.), px(0.)), size(px(900.), px(600.)), |_, _| {
            view.clone()
        });
        assert!(visual_cx.debug_bounds("service-add").is_some());
    }

    #[gpui::test]
    fn cleanup_page_renders(cx: &mut TestAppContext) {
        let state = cx.new(|_| {
            let mut state = AppState::new();
            state.set_view(View::Cleanup);
            state
        });
        let (view, visual_cx) = cx.add_window_view(|_, cx| MainWindow::new(state.clone(), cx));
        let _ = visual_cx.draw(point(px(0.), px(0.)), size(px(1100.), px(720.)), |_, _| {
            view.clone()
        });
        assert!(visual_cx.debug_bounds("cleanup-scan").is_some());
    }

    #[gpui::test]
    fn service_detail_renders(cx: &mut TestAppContext) {
        let state = cx.new(|_| {
            let mut state = AppState::new();
            state.begin_service_detail("Demo");
            state.set_service_detail(Ok(app_service::ServiceDetails {
                name: "Demo".into(),
                display_name: "Demo Service".into(),
                description: "A demo service".into(),
                status: "Running".into(),
                start_type: app_service::StartType::Automatic,
                binary_path: r"C:\demo.exe".into(),
                account: "LocalSystem".into(),
                process_id: 1234,
                depends_on: vec!["Dep".into()],
                dependents: Vec::new(),
            }));
            state
        });
        let (view, visual_cx) = cx.add_window_view(|_, cx| MainWindow::new(state.clone(), cx));
        let _ = visual_cx.draw(point(px(0.), px(0.)), size(px(1100.), px(720.)), |_, _| {
            view.clone()
        });
        assert!(visual_cx.debug_bounds("detail-close").is_some());
    }

    #[gpui::test]
    fn titlebar_mouse_down_does_not_prevent_native_window_drag(cx: &mut TestAppContext) {
        let state = cx.new(|_| AppState::new());
        let (view, visual_cx) = cx.add_window_view(|_, cx| MainWindow::new(state.clone(), cx));
        let _ = visual_cx.draw(point(px(0.), px(0.)), size(px(1100.), px(720.)), |_, _| {
            view.clone()
        });

        visual_cx.simulate_mouse_move(point(px(500.), px(24.)), None, Modifiers::default());
        visual_cx.simulate_mouse_down(
            point(px(500.), px(24.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        let default_prevented = visual_cx.update(|window, _| window.default_prevented());

        assert!(!default_prevented);
    }

    #[test]
    fn service_status_column_is_compact() {
        assert_eq!(SERVICE_STATUS_COLUMN_WIDTH, 96.);
    }

    #[test]
    fn tools_catalog_contains_only_system_info() {
        assert_eq!(TOOLS.len(), 1);
        assert_eq!(TOOLS[0], ("tool_sysinfo", "tool_sysinfo_desc"));
    }
}
