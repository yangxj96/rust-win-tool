use gpui::{
    div, prelude::*, px, rgb, AppContext, ClickEvent, Context, Div, Entity, FocusHandle,
    KeyDownEvent, Render, Window, WindowControlArea,
};

use crate::app::{AppState, OperationState, PendingAction, ServiceStatus, View};
use crate::backend::{self, ServiceOperation};
use crate::ui::{components, input};

const TOOLS: [(&str, &str); 1] = [("tool_sysinfo", "tool_sysinfo_desc")];
const SERVICE_STATUS_COLUMN_WIDTH: f32 = 80.;

pub struct MainWindow {
    pub(crate) state: Entity<AppState>,
    pub(crate) focus_handle: FocusHandle,
}

impl MainWindow {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        Self {
            state,
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.state.read(cx).show_add_dialog() {
            self.handle_dialog_key(event, cx);
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

    fn handle_dialog_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        match input::text_editing_input(event) {
            Some(input::TextEditingInput::Backspace) => {
                self.update_state(cx, |state| state.add_dialog_backspace());
            }
            Some(input::TextEditingInput::Text(value)) => {
                self.update_state(cx, move |state| {
                    for character in value.chars() {
                        state.add_dialog_input(character);
                    }
                });
            }
            None => {}
        }
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

    fn select_setting(
        &mut self,
        index: usize,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, move |state| state.select_setting(index));
    }

    fn begin_add_dialog(&mut self, cx: &mut Context<Self>) {
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
        })
        .detach();
    }

    pub(crate) fn refresh_services(&mut self, cx: &mut Context<Self>) {
        let action = self.state.update(cx, |state, cx| {
            let action = state.request_refresh();
            cx.notify();
            action
        });
        self.spawn_service_operation(action, cx);
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
            self.spawn_service_operation(action, cx);
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
            self.spawn_service_operation(action, cx);
        }
    }

    fn start_all(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).current_view() == View::Service {
            let action = self.state.update(cx, |state, cx| {
                let action = state.request_start_all();
                cx.notify();
                action
            });
            self.spawn_service_operation(action, cx);
        }
    }

    fn stop_all(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).current_view() == View::Service {
            let action = self.state.update(cx, |state, cx| {
                let action = state.request_stop_all();
                cx.notify();
                action
            });
            self.spawn_service_operation(action, cx);
        }
    }

    fn spawn_service_operation(&mut self, action: PendingAction, cx: &mut Context<Self>) {
        let operation = match action {
            PendingAction::StartService(name) => ServiceOperation::Start(name),
            PendingAction::StopService(name) => ServiceOperation::Stop(name),
            PendingAction::StartAll(names) => ServiceOperation::StartAll(names),
            PendingAction::StopAll(names) => ServiceOperation::StopAll(names),
            PendingAction::RefreshAll(names) => ServiceOperation::RefreshAll(names),
        };
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let results = cx
                .background_spawn(async move { backend::execute_service_operation(operation) })
                .await;
            state
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
                    cx.notify();
                })
                .ok();
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
            .h(px(48.))
            .flex_shrink_0()
            .bg(components::color(colors.bg_surface))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .flex_shrink_0()
                    .window_control_area(WindowControlArea::Drag)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(26.))
                            .h(px(26.))
                            .rounded_md()
                            .bg(components::color(colors.primary))
                            .text_color(components::color(0xffffff))
                            .text_sm()
                            .child("R"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(components::color(colors.fg_default))
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
        let navigation: [(View, &str); 4] = [
            (View::Service, translations.tab_service),
            (View::Tools, translations.tab_tools),
            (View::Scripts, translations.tab_scripts),
            (View::Settings, translations.tab_settings),
        ];

        div()
            .flex()
            .flex_col()
            .w(px(220.))
            .h_full()
            .flex_shrink_0()
            .p_3()
            .gap_1()
            .bg(components::color(state.theme_colors().bg_sidebar))
            .border_r_1()
            .border_color(components::color(state.theme_colors().border))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .w_full()
                    .h(px(64.))
                    .px_2()
                    .mb_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(34.))
                            .h(px(34.))
                            .rounded_lg()
                            .bg(components::color(state.theme_colors().primary))
                            .text_color(components::color(0xffffff))
                            .text_lg()
                            .child("R"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(components::color(state.theme_colors().fg_default))
                            .child(translations.app_title),
                    ),
            )
            .child(
                div()
                    .px_2()
                    .pb_2()
                    .text_xs()
                    .text_color(components::color(state.theme_colors().inactive))
                    .child("NAVIGATION"),
            )
            .children(navigation.into_iter().map(|(view, label)| {
                let selected = state.current_view() == view;
                let id = match view {
                    View::Service => "sidebar-service",
                    View::Tools => "sidebar-tools",
                    View::Scripts => "sidebar-scripts",
                    View::Settings => "sidebar-settings",
                };
                components::sidebar_item(label.trim(), state.theme_colors(), selected)
                    .id(id)
                    .on_click(cx.listener(move |this, event, window, cx| {
                        this.select_view(view, event, window, cx)
                    }))
            }))
    }

    fn render_content_header(&self, state: &AppState) -> Div {
        let translations = state.t();
        let (title, description) =
            if state.current_view() == View::Tools && state.tool_detail_active() {
                (translations.sysinfo_title, translations.tool_sysinfo_desc)
            } else {
                match state.current_view() {
                    View::Service => (translations.svc_header, translations.page_service_desc),
                    View::Tools => (translations.tools_header, translations.page_tools_desc),
                    View::Scripts => (translations.scripts_header, translations.page_scripts_desc),
                    View::Settings => (
                        translations.settings_header,
                        translations.page_settings_desc,
                    ),
                }
            };
        div()
            .flex()
            .items_center()
            .w_full()
            .h(px(84.))
            .px_6()
            .flex_shrink_0()
            .border_b_1()
            .border_color(components::color(state.theme_colors().border))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_2xl()
                            .text_color(components::color(state.theme_colors().fg_default))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(components::color(state.theme_colors().inactive))
                            .child(description),
                    ),
            )
    }

    fn render_service_page(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let primary_actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                components::button(translations.hint_add, colors, true)
                    .id("service-add")
                    .on_click(cx.listener(Self::add_clicked)),
            )
            .child(
                components::button(translations.hint_refresh, colors, false)
                    .id("service-refresh")
                    .on_click(cx.listener(Self::refresh_clicked)),
            );
        let batch_actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                components::button(translations.hint_start_all, colors, false)
                    .id("service-start-all")
                    .on_click(cx.listener(Self::start_all_clicked)),
            )
            .child(
                components::button(translations.hint_stop_all, colors, false)
                    .id("service-stop-all")
                    .on_click(cx.listener(Self::stop_all_clicked)),
            );
        let toolbar = div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .mb_4()
            .child(primary_actions)
            .child(batch_actions);

        let header = div()
            .flex()
            .items_center()
            .w_full()
            .h(px(40.))
            .px_3()
            .rounded_md()
            .bg(components::color(colors.bg_window))
            .text_sm()
            .text_color(components::color(colors.inactive))
            .child(div().w(px(170.)).child(translations.col_name))
            .child(div().w(px(180.)).child(translations.col_display))
            .child(
                div()
                    .w(px(SERVICE_STATUS_COLUMN_WIDTH))
                    .child(translations.col_status),
            )
            .child(div().flex_1().child(translations.col_message))
            .child(div().w(px(150.)).child(translations.col_actions));

        let rows = state
            .managed_services()
            .iter()
            .enumerate()
            .map(|(index, service)| {
                let status = state.service_status(&service.name);
                let message = state.service_message(&service.name).unwrap_or("");
                let service_name = service.name.clone();
                let start_name = service_name.clone();
                let stop_name = service_name.clone();
                let delete_name = service_name;
                let actions = div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .w(px(150.))
                    .child(
                        components::button(translations.hint_start, colors, false)
                            .px_1()
                            .id(("service-start", index))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.start_service(start_name.clone(), cx)
                            })),
                    )
                    .child(
                        components::button(translations.hint_stop, colors, false)
                            .px_1()
                            .id(("service-stop", index))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.stop_service(stop_name.clone(), cx)
                            })),
                    )
                    .child(
                        components::button(translations.hint_delete, colors, false)
                            .px_1()
                            .id(("service-delete", index))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let delete_name = delete_name.clone();
                                this.update_state(cx, move |state| {
                                    state.remove_service(&delete_name)
                                })
                            })),
                    );
                div()
                    .flex()
                    .items_center()
                    .w_full()
                    .min_h(px(52.))
                    .px_3()
                    .border_b_1()
                    .border_color(components::color(colors.border))
                    .bg(components::color(colors.bg_surface))
                    .text_color(components::color(colors.fg_default))
                    .hover(move |style| style.bg(components::color(colors.bg_window)))
                    .id(("service-row", index))
                    .child(div().w(px(170.)).child(service.name.clone()))
                    .child(div().w(px(180.)).child(service.display_name.clone()))
                    .child(div().w(px(SERVICE_STATUS_COLUMN_WIDTH)).child(
                        components::status_badge(
                            status_label(status, translations),
                            status_color(status, colors),
                        ),
                    ))
                    .child(
                        div()
                            .flex_1()
                            .text_color(components::color(if message.is_empty() {
                                colors.inactive
                            } else {
                                colors.error
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
                .items_center()
                .justify_center()
                .text_color(components::color(colors.inactive))
                .id("service-empty")
                .child(translations.svc_empty)
        } else {
            components::card(colors)
                .flex()
                .flex_col()
                .gap_1()
                .w_full()
                .flex_1()
                .p_3()
                .child(header)
                .children(rows)
                .id("service-list")
                .overflow_y_scroll()
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_6()
            .child(toolbar)
            .child(body)
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
                    .flex_col()
                    .gap_1()
                    .p_4()
                    .border_color(components::color(if selected {
                        colors.primary
                    } else {
                        colors.border
                    }))
                    .bg(if selected {
                        components::color(colors.bg_select)
                    } else {
                        components::color(colors.bg_surface)
                    })
                    .text_color(components::color(colors.fg_default))
                    .cursor_pointer()
                    .id(("tool-row", index))
                    .on_click(cx.listener(move |this, event, window, cx| {
                        this.select_tool(index, event, window, cx);
                        if index == 0 {
                            this.open_tool(cx);
                        }
                    }))
                    .child(div().text_lg().child(label))
                    .child(
                        div()
                            .text_sm()
                            .text_color(components::color(colors.inactive))
                            .child(description),
                    )
            });
        div().flex().flex_col().gap_3().size_full().p_6().child(
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
                .items_center()
                .justify_center()
                .text_color(components::color(colors.inactive))
                .child(translations.sysinfo_fetching)
        } else if let Some(info) = state.system_info() {
            let rows = [
                (translations.sysinfo_os, info.os.as_str()),
                (translations.sysinfo_version, info.version.as_str()),
                (translations.sysinfo_build, info.build.as_str()),
                (translations.sysinfo_computer, info.computer.as_str()),
                (translations.sysinfo_user, info.user.as_str()),
                (translations.sysinfo_cpu, info.cpu.as_str()),
                (translations.sysinfo_cores, info.cores.as_str()),
                (translations.sysinfo_ram, info.ram.as_str()),
            ];
            components::card(colors)
                .flex()
                .flex_col()
                .gap_2()
                .w_full()
                .flex_1()
                .p_3()
                .children(rows.into_iter().map(|(label, value)| {
                    div()
                        .flex()
                        .w_full()
                        .p_3()
                        .rounded_md()
                        .bg(components::color(colors.bg_window))
                        .border_1()
                        .border_color(components::color(colors.border))
                        .child(
                            div()
                                .w(px(180.))
                                .text_color(components::color(colors.inactive))
                                .child(label),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_color(components::color(colors.fg_default))
                                .child(value.to_string()),
                        )
                }))
        } else {
            div()
                .flex_1()
                .items_center()
                .justify_center()
                .text_color(components::color(colors.error))
                .child(translations.err_failed)
        };
        div()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .p_6()
            .child(
                div().flex().justify_end().child(
                    components::button(translations.sysinfo_back, colors, false)
                        .id("sysinfo-back")
                        .on_click(cx.listener(|this, _event, _window, cx| {
                            this.update_state(cx, |state| state.close_tool_detail());
                        })),
                ),
            )
            .child(content)
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
                div().flex_1().child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_lg().child(translations.script_reset_navicat))
                        .child(
                            div()
                                .text_sm()
                                .text_color(components::color(colors.inactive))
                                .child(translations.script_reset_navicat_desc),
                        ),
                ),
            )
            .child(components::button(translations.hint_confirm, colors, true).id("script-run"));
        let mut page = div().flex().flex_col().gap_4().size_full().p_6().child(row);
        if let Some(result) = state.script_result() {
            page = page.child(
                components::card(colors)
                    .w_full()
                    .p_3()
                    .bg(components::color(colors.bg_select))
                    .border_color(components::color(colors.primary))
                    .text_color(components::color(colors.accent))
                    .child(result.to_string()),
            );
        }
        page
    }

    fn render_settings_page(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let rows = [
            (
                translations.setting_language,
                state.language().name(),
                0usize,
            ),
            (
                translations.setting_theme,
                match state.theme() {
                    crate::app::Theme::Dark => translations.theme_dark,
                    crate::app::Theme::Light => translations.theme_light,
                },
                1usize,
            ),
        ];
        div().flex().flex_col().gap_3().size_full().p_6().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .w_full()
                .flex_1()
                .children(rows.into_iter().map(|(label, value, index)| {
                    let selected = state.settings_selected() == index;
                    components::card(colors)
                        .flex()
                        .items_center()
                        .p_4()
                        .border_color(components::color(if selected {
                            colors.primary
                        } else {
                            colors.border
                        }))
                        .bg(if selected {
                            components::color(colors.bg_select)
                        } else {
                            components::color(colors.bg_surface)
                        })
                        .cursor_pointer()
                        .id(("setting-row", index))
                        .on_click(cx.listener(move |this, event, window, cx| {
                            this.select_setting(index, event, window, cx);
                            this.update_state(cx, move |state| {
                                if index == 0 {
                                    state.cycle_language();
                                } else {
                                    state.cycle_theme();
                                }
                            });
                        }))
                        .child(div().flex_1().child(label))
                        .child(
                            div()
                                .text_color(components::color(colors.accent))
                                .child(value),
                        )
                })),
        )
    }

    fn render_add_dialog(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let list = if state.add_dialog_loading() {
            div()
                .flex_1()
                .items_center()
                .justify_center()
                .child(translations.status_refreshing)
                .id("dialog-loading")
        } else if state.add_dialog_filtered().is_empty() {
            div()
                .flex_1()
                .items_center()
                .justify_center()
                .child(translations.dialog_empty)
                .id("dialog-empty")
        } else {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .flex_1()
                .children(state.add_dialog_filtered().iter().enumerate().map(
                    |(index, &real_index)| {
                        let service = &state.add_dialog_services()[real_index];
                        let selected = state.add_dialog_selected() == index;
                        div()
                            .flex()
                            .items_center()
                            .w_full()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(components::color(if selected {
                                colors.primary
                            } else {
                                colors.border
                            }))
                            .bg(if selected {
                                components::color(colors.bg_select)
                            } else {
                                components::color(colors.bg_surface)
                            })
                            .cursor_pointer()
                            .id(("dialog-service", index))
                            .on_click(cx.listener(move |this, event, window, cx| {
                                this.select_dialog_service(index, event, window, cx);
                            }))
                            .child(
                                div()
                                    .flex_1()
                                    .child(format!("{} — {}", service.name, service.display_name)),
                            )
                    },
                ))
                .id("dialog-service-list")
                .overflow_y_scroll()
        };
        let error = state.add_dialog_error().map(|error| {
            div()
                .text_color(components::color(colors.error))
                .child(error.to_string())
        });
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(0x99000000))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .w(px(620.))
                    .h(px(500.))
                    .p_5()
                    .rounded_lg()
                    .shadow_lg()
                    .bg(components::color(colors.bg_surface))
                    .border_1()
                    .border_color(components::color(colors.border))
                    .text_color(components::color(colors.fg_default))
                    .child(
                        div().flex().items_center().justify_between().child(
                            div()
                                .text_xl()
                                .text_color(components::color(colors.fg_default))
                                .child(translations.dialog_add_title),
                        ),
                    )
                    .child(components::search_field(
                        state.add_dialog_search(),
                        translations.dialog_search,
                        colors,
                    ))
                    .children(error)
                    .child(list)
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                components::button(translations.dialog_hint_cancel, colors, false)
                                    .id("dialog-cancel")
                                    .on_click(cx.listener(Self::cancel_dialog_clicked)),
                            )
                            .child(
                                components::button(translations.dialog_hint_add, colors, true)
                                    .id("dialog-confirm")
                                    .on_click(cx.listener(Self::confirm_dialog_clicked)),
                            ),
                    ),
            )
    }

    fn add_clicked(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.begin_add_dialog(cx);
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

    fn script_clicked(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.run_script(cx);
    }

    fn cancel_dialog_clicked(
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
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let page = match state.current_view() {
            View::Service => self.render_service_page(state, cx),
            View::Tools => self.render_tools_page(state, cx),
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
            content = content.child(self.render_add_dialog(state, cx));
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(components::color(state.theme_colors().bg_window))
            .text_color(components::color(state.theme_colors().fg_default))
            .child(self.render_titlebar(window, state, cx))
            .child(content)
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

fn status_label(status: ServiceStatus, translations: &crate::i18n::Translations) -> &'static str {
    match status {
        ServiceStatus::Running => translations.status_running,
        ServiceStatus::Stopped | ServiceStatus::Unknown => translations.status_stopped,
        ServiceStatus::Starting => translations.status_starting,
        ServiceStatus::Stopping => translations.status_stopping,
        ServiceStatus::Refreshing => translations.status_refreshing,
    }
}

fn status_color(status: ServiceStatus, colors: &crate::theme::ThemeColors) -> u32 {
    match status {
        ServiceStatus::Running => colors.success,
        ServiceStatus::Stopped => colors.danger,
        ServiceStatus::Starting | ServiceStatus::Stopping | ServiceStatus::Refreshing => {
            colors.warning
        }
        ServiceStatus::Unknown => colors.inactive,
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
    use crate::app::AppState;
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
        assert!(visual_cx.debug_bounds("service-summary").is_none());
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
        assert_eq!(SERVICE_STATUS_COLUMN_WIDTH, 80.);
    }

    #[test]
    fn tools_catalog_contains_only_system_info() {
        assert_eq!(TOOLS.len(), 1);
        assert_eq!(TOOLS[0], ("tool_sysinfo", "tool_sysinfo_desc"));
    }
}
