//! 设置页面，提供持久化的语言和主题选项。

use super::shared::*;
use super::*;

impl MainWindow {
    pub(super) fn render_settings_page(&self, state: &AppState, cx: &Context<Self>) -> Div {
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

        let logs_row = components::card(colors)
            .flex()
            .items_center()
            .justify_between()
            .p_4()
            .id("setting-row-2")
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(components::color(colors.fg.default))
                    .child(translations.setting_logs),
            )
            .child(
                components::button(
                    translations.hint_open_dir,
                    colors,
                    components::ButtonVariant::Secondary,
                )
                .id("setting-open-logs")
                .debug_selector(|| "setting-open-logs".to_string())
                .on_click(cx.listener(|_, _, _, _| crate::logging::open_dir())),
            );

        div()
            .flex()
            .flex_col()
            .gap_3()
            .size_full()
            .p_6()
            .child(language_row)
            .child(theme_row)
            .child(logs_row)
    }
}
