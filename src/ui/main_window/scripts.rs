//! Custom script list, editor, execution callbacks, and output dialog.
//!
//! Script persistence and execution privileges remain defined by the existing
//! app and backend layers.

use super::shared::*;
use super::*;

impl MainWindow {
    pub(super) fn render_scripts_page(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();

        let rows = SCRIPTS
            .iter()
            .enumerate()
            .map(|(index, (name, description))| {
                let label = script_text(translations, name);
                let description = script_text(translations, description);
                let selected = state.scripts_selected() == index;
                components::card(colors)
                    .flex()
                    .items_center()
                    .gap_3()
                    .w_full()
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
                    .id(("script-row", index))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.update_state(cx, move |state| state.select_script(index))
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
                                    .child(label),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(components::color(colors.fg.muted))
                                    .child(description),
                            ),
                    )
            });

        let toolbar = div()
            .flex()
            .items_center()
            .gap_3()
            .w_full()
            .child(
                components::button(
                    translations.script_run,
                    colors,
                    components::ButtonVariant::Primary,
                )
                .id("script-run")
                .on_click(cx.listener(Self::script_clicked)),
            )
            .child(
                components::button(
                    translations.script_add,
                    colors,
                    components::ButtonVariant::Secondary,
                )
                .id("script-add")
                .on_click(cx.listener(Self::add_script_clicked)),
            )
            .children(state.script_running().then(|| {
                div()
                    .text_sm()
                    .text_color(components::color(colors.fg.muted))
                    .child(translations.status_refreshing)
            }));

        let custom_rows = state
            .custom_scripts()
            .iter()
            .enumerate()
            .map(|(index, script)| {
                let row_index = SCRIPTS.len() + index;
                let selected = state.scripts_selected() == row_index;
                components::card(colors)
                    .flex()
                    .items_center()
                    .gap_3()
                    .w_full()
                    .p_3()
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
                    .id(("custom-script-row", index))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.update_state(cx, move |state| state.select_script(row_index))
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(components::color(colors.fg.default))
                                    .child(script.name.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(components::color(colors.fg.muted))
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .child(script.command.clone()),
                            ),
                    )
                    .child(
                        components::button(
                            translations.hint_delete,
                            colors,
                            components::ButtonVariant::Danger,
                        )
                        .h(px(26.))
                        .px_2()
                        .text_xs()
                        .id(("custom-script-delete", index))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.update_state(cx, move |state| state.remove_custom_script(index))
                        })),
                    )
            });

        div()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .p_6()
            .child(toolbar)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .w_full()
                    .flex_1()
                    .min_h(px(0.))
                    .id("script-list")
                    .overflow_y_scroll()
                    .child(div().flex().flex_col().gap_3().w_full().children(rows))
                    .children(
                        (!state.custom_scripts().is_empty())
                            .then(|| components::section_label(translations.script_custom, colors)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .w_full()
                            .children(custom_rows),
                    ),
            )
    }

    pub(super) fn render_add_script_dialog(
        &self,
        window: &Window,
        state: &AppState,
        cx: &Context<Self>,
    ) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let name_focused = self.script_name_focus.is_focused(window);
        let content_focused = self.script_content_focus.is_focused(window);
        let kind = state.script_dialog_kind();

        let name_input = {
            let input_entity = cx.entity();
            let input_focus = self.script_name_focus.clone();
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
        let name_field = components::search_field(
            state.script_dialog_name(),
            state.script_dialog_name_marked(),
            "",
            colors,
            name_focused,
        )
        .relative()
        .track_focus(&self.script_name_focus)
        .cursor_text()
        .focus(|style| style.border_color(components::color(colors.brand.primary)))
        .id("script-name")
        .on_click(cx.listener(Self::focus_script_name))
        .child(name_input);

        let content_input = {
            let input_entity = cx.entity();
            let input_focus = self.script_content_focus.clone();
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
        let content_field = components::text_area(
            state.script_dialog_content(),
            state.script_dialog_content_marked(),
            translations.script_placeholder,
            colors,
            content_focused,
            168.,
        )
        .relative()
        .track_focus(&self.script_content_focus)
        .cursor_text()
        .focus(|style| style.border_color(components::color(colors.brand.primary)))
        .id("script-content")
        .on_click(cx.listener(Self::focus_script_content))
        .child(content_input);

        let label = move |text: &str| -> Div {
            div()
                .w(px(64.))
                .flex_shrink_0()
                .pt(px(8.))
                .text_sm()
                .text_color(components::color(colors.fg.muted))
                .child(text.to_string())
        };

        let error = state
            .script_dialog_error()
            .map(|text| components::alert(text.to_string(), &colors.danger));

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
                    .w(px(520.))
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
                            .child(translations.script_add_title),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .w_full()
                            .child(label(translations.script_name_label))
                            .child(div().flex_1().min_w(px(0.)).child(name_field)),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .w_full()
                            .child(label(translations.script_type_label))
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(
                                        segment(
                                            translations.script_kind_cmd,
                                            kind == ScriptKind::Cmd,
                                            colors,
                                            "script-kind-cmd",
                                        )
                                        .on_click(cx.listener(Self::script_kind_cmd_clicked)),
                                    )
                                    .child(
                                        segment(
                                            translations.script_kind_powershell,
                                            kind == ScriptKind::PowerShell,
                                            colors,
                                            "script-kind-powershell",
                                        )
                                        .on_click(
                                            cx.listener(Self::script_kind_powershell_clicked),
                                        ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .w_full()
                            .child(label(translations.script_content_label))
                            .child(div().flex_1().min_w(px(0.)).child(content_field)),
                    )
                    .children(error)
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
                                .id("script-dialog-cancel")
                                .on_click(cx.listener(Self::cancel_add_script_clicked)),
                            )
                            .child(
                                components::button(
                                    translations.dialog_hint_add,
                                    colors,
                                    components::ButtonVariant::Primary,
                                )
                                .id("script-dialog-add")
                                .on_click(cx.listener(Self::confirm_add_script_clicked)),
                            ),
                    ),
            )
    }

    pub(super) fn script_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.run_script(cx);
    }

    pub(super) fn add_script_clicked(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.begin_add_script());
        window.focus(&self.script_name_focus);
    }

    pub(super) fn focus_script_name(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.script_name_focus);
        cx.notify();
    }

    pub(super) fn focus_script_content(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.script_content_focus);
        cx.notify();
    }

    pub(super) fn script_kind_cmd_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.set_script_dialog_kind(ScriptKind::Cmd));
    }

    pub(super) fn script_kind_powershell_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| {
            state.set_script_dialog_kind(ScriptKind::PowerShell)
        });
    }

    pub(super) fn confirm_add_script_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.confirm_add_script());
    }

    pub(super) fn cancel_add_script_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.close_add_script());
    }

    pub(super) fn close_script_output_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.close_script_output());
    }

    /// Modal that shows the output of the most recent script run. Closing it
    /// leaves the script list unobstructed.
    pub(super) fn render_script_output(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let text = state.script_output().unwrap_or_default().to_string();

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
                    .w(px(620.))
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
                                    .child(translations.script_output_title),
                            )
                            .child(
                                components::button("✕", colors, components::ButtonVariant::Ghost)
                                    .w(px(28.))
                                    .h(px(28.))
                                    .px_0()
                                    .id("script-output-close-icon")
                                    .on_click(cx.listener(Self::close_script_output_clicked)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .w_full()
                            .flex_1()
                            .min_h(px(0.))
                            .overflow_hidden()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .w_full()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .min_h(px(0.))
                                    .id("script-output-body")
                                    .overflow_y_scroll()
                                    .track_scroll(&self.script_scroll)
                                    .on_scroll_wheel(cx.listener(Self::on_scrolled))
                                    .text_sm()
                                    .whitespace_normal()
                                    .text_color(components::color(colors.fg.default))
                                    .child(text),
                            )
                            .child(self.render_scrollbar(
                                &self.script_scroll,
                                colors,
                                cx,
                                "script-scrollbar",
                                "script-thumb",
                            )),
                    )
                    .child(
                        div().flex().justify_end().child(
                            components::button(
                                translations.dialog_close,
                                colors,
                                components::ButtonVariant::Secondary,
                            )
                            .id("script-output-close")
                            .on_click(cx.listener(Self::close_script_output_clicked)),
                        ),
                    ),
            )
    }
}
