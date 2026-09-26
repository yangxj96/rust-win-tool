//! Main window chrome: titlebar, navigation sidebar, content header, and scrollbars.

use super::*;

impl MainWindow {
    pub(super) fn render_titlebar(
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

    pub(super) fn render_sidebar(&self, state: &AppState, cx: &Context<Self>) -> Div {
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

    pub(super) fn render_content_header(&self, state: &AppState) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let (title, description) =
            if state.current_view() == View::Tools && state.tool_detail_active() {
                match state.tools_selected() {
                    1 => (
                        translations.tool_processes,
                        translations.tool_processes_desc,
                    ),
                    2 => (translations.tool_monitor, translations.tool_monitor_desc),
                    3 => (translations.tool_network, translations.tool_network_desc),
                    4 => (translations.tool_startup, translations.tool_startup_desc),
                    _ => (translations.sysinfo_title, translations.tool_sysinfo_desc),
                }
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
    pub(super) fn render_scrollbar(
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

    pub(super) fn on_scrollbar_drag(
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
    pub(super) fn on_scrolled(
        &mut self,
        _event: &gpui::ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.notify();
    }
}
