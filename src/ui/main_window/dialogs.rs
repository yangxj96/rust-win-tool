//! 服务详情、操作历史、工具、脚本输出和操作确认等共用对话框。
//!
//! 每个待处理操作都保留独立的确认与取消回调。

use super::shared::*;
use super::*;
use crate::cleanup as cleanup_backend;

impl MainWindow {
    pub(super) fn render_add_dialog(
        &self,
        window: &Window,
        state: &AppState,
        cx: &Context<Self>,
    ) -> Div {
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
                .min_h(px(0.))
                .overflow_hidden()
                .id("dialog-scroll-area")
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .flex_1()
                        .min_w(px(0.))
                        .min_h(px(0.))
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

        // 搜索框是此窗口唯一的文本输入项。透明画布覆盖层在每次绘制时注册平台输入处理器，
        // 以支持 Windows 输入法组合输入（例如中文输入）。
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
            .occlude()
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

    pub(super) fn cancel_dialog_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.close_add_dialog());
    }

    pub(super) fn close_dialog_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.close_add_dialog());
    }

    pub(super) fn confirm_dialog_clicked(
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

    pub(super) fn render_service_detail(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let name = state.service_detail_name().unwrap_or_default().to_string();
        let display_name = state
            .service_detail()
            .map(|details| {
                if details.display_name.trim().is_empty() || details.display_name == details.name {
                    details.name.clone()
                } else {
                    format!("{} ({})", details.display_name, details.name)
                }
            })
            .unwrap_or_else(|| name.clone());

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
            .occlude()
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
                                            .child(display_name),
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
                            .min_h(px(0.))
                            .id("detail-body")
                            .overflow_y_scroll()
                            .child(body),
                    ),
            )
    }

    pub(super) fn render_history(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let entries = state.history();

        let list: AnyElement = if entries.is_empty() {
            div()
                .flex()
                .items_center()
                .justify_center()
                .py_6()
                .w_full()
                .text_color(components::color(colors.fg.muted))
                .child(translations.history_empty)
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .w_full()
                .children(entries.iter().rev().map(|entry| {
                    let accent = if entry.ok {
                        colors.success.text
                    } else {
                        colors.danger.text
                    };
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .w_full()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(components::color(colors.bg.canvas))
                        .text_sm()
                        .child(
                            div()
                                .w(px(14.))
                                .text_color(components::color(accent))
                                .child(if entry.ok { "✓" } else { "✗" }),
                        )
                        .child(
                            div()
                                .w(px(84.))
                                .flex_shrink_0()
                                .text_color(components::color(colors.fg.muted))
                                .child(history_action_label(entry.action, translations)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_color(components::color(colors.fg.default))
                                .child(entry.service.clone()),
                        )
                        .children((!entry.message.is_empty()).then(|| {
                            div()
                                .flex_shrink_0()
                                .text_color(components::color(colors.danger.text))
                                .child(entry.message.clone())
                        }))
                }))
                .into_any_element()
        };

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .occlude()
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
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(translations.history_title),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        components::button(
                                            translations.history_clear,
                                            colors,
                                            components::ButtonVariant::Ghost,
                                        )
                                        .h(px(28.))
                                        .px_2()
                                        .text_xs()
                                        .id("history-clear")
                                        .on_click(cx.listener(Self::clear_history_clicked)),
                                    )
                                    .child(
                                        components::button(
                                            "✕",
                                            colors,
                                            components::ButtonVariant::Ghost,
                                        )
                                        .w(px(28.))
                                        .h(px(28.))
                                        .px_0()
                                        .id("history-close")
                                        .on_click(cx.listener(Self::close_history_clicked)),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .w_full()
                            .flex_1()
                            .min_h(px(0.))
                            .id("history-body")
                            .overflow_y_scroll()
                            .child(list),
                    ),
            )
    }

    pub(super) fn render_confirm_dialog(
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
            .occlude()
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

    pub(super) fn render_confirm_kill(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let (pid, name) = self.pending_kill.clone().unwrap_or((0, String::new()));
        let message = translations
            .proc_confirm_message
            .replacen("{}", &name, 1)
            .replacen("{}", &pid.to_string(), 1);
        self.render_confirm_dialog(
            state,
            cx,
            ConfirmDialog {
                title: translations.proc_confirm_title,
                message,
                ok_label: translations.proc_end,
                ok_id: "confirm-kill-ok",
                cancel_id: "confirm-kill-cancel",
                danger: true,
                on_ok: Self::confirm_kill_clicked,
                on_cancel: Self::cancel_kill_clicked,
            },
        )
    }

    pub(super) fn render_confirm_delete(&self, state: &AppState, cx: &Context<Self>) -> Div {
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

    pub(super) fn render_confirm_clean(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let totals = state.selected_cleanup_totals();
        let mode = match state.cleanup().delete_mode {
            DeleteMode::Recycle => translations.cleanup_mode_recycle,
            DeleteMode::Permanent => translations.cleanup_mode_permanent,
        };
        let message = translations
            .cleanup_confirm_message
            .replacen("{}", &cleanup_backend::format_bytes(totals.bytes), 1)
            .replacen("{}", &totals.files.to_string(), 1)
            .replacen("{}", mode, 1);
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

    pub(super) fn render_confirm_empty_bin(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let (bytes, items) = state.cleanup().recycle_bin.unwrap_or((0, 0));
        let message = translations
            .cleanup_bin_confirm_message
            .replacen("{}", &items.to_string(), 1)
            .replacen("{}", &cleanup_backend::format_bytes(bytes), 1);
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
