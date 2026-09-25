use gpui::prelude::*;
use gpui::{px, rgb, Div, Rgba, SharedString};

use crate::theme::ThemeColors;

pub fn color(value: u32) -> Rgba {
    rgb(value)
}

pub fn button(label: impl Into<SharedString>, colors: &ThemeColors, primary: bool) -> Div {
    let background = if primary {
        colors.primary
    } else {
        colors.bg_surface
    };
    let foreground = if primary { 0xffffff } else { colors.fg_default };
    gpui::div()
        .flex()
        .items_center()
        .justify_center()
        .h(px(34.))
        .px_3()
        .rounded_md()
        .bg(color(background))
        .border_1()
        .border_color(color(if primary {
            colors.primary
        } else {
            colors.border
        }))
        .text_color(color(foreground))
        .cursor_pointer()
        .hover(move |style| {
            style
                .bg(color(if primary {
                    colors.primary
                } else {
                    colors.bg_select
                }))
                .opacity(0.9)
        })
        .child(label.into())
}

pub fn card(colors: &ThemeColors) -> Div {
    gpui::div()
        .w_full()
        .rounded_lg()
        .border_1()
        .border_color(color(colors.border))
        .bg(color(colors.bg_surface))
}

#[cfg(test)]
pub fn stat_card(
    label: impl Into<SharedString>,
    value: usize,
    accent: u32,
    colors: &ThemeColors,
) -> Div {
    gpui::div()
        .flex()
        .items_center()
        .justify_between()
        .flex_1()
        .min_h(px(74.))
        .p_4()
        .rounded_lg()
        .border_1()
        .border_color(color(colors.border))
        .bg(color(colors.bg_surface))
        .child(
            gpui::div()
                .text_sm()
                .text_color(color(colors.inactive))
                .child(label.into()),
        )
        .child(
            gpui::div()
                .text_2xl()
                .text_color(color(accent))
                .child(value.to_string()),
        )
}

pub fn sidebar_item(label: impl Into<SharedString>, colors: &ThemeColors, selected: bool) -> Div {
    let background = if selected {
        colors.bg_select
    } else {
        colors.bg_sidebar
    };
    let foreground = if selected {
        colors.primary
    } else {
        colors.fg_default
    };
    let accent = if selected {
        colors.primary
    } else {
        colors.bg_sidebar
    };
    gpui::div()
        .flex()
        .items_center()
        .w_full()
        .h(px(42.))
        .px_3()
        .rounded_md()
        .bg(color(background))
        .text_color(color(foreground))
        .border_l_3()
        .border_color(color(accent))
        .cursor_pointer()
        .hover(move |style| style.bg(color(colors.bg_select)))
        .child(label.into())
}

pub fn window_control(label: impl Into<SharedString>, colors: &ThemeColors, danger: bool) -> Div {
    let hover_background = if danger {
        colors.danger
    } else {
        colors.bg_select
    };
    let hover_foreground = if danger { 0xffffff } else { colors.fg_default };
    gpui::div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(46.))
        .h(px(48.))
        .text_color(color(colors.inactive))
        .cursor_pointer()
        .hover(move |style| {
            style
                .bg(color(hover_background))
                .text_color(color(hover_foreground))
        })
        .child(label.into())
}

pub fn status_badge(label: impl Into<SharedString>, status_color: u32) -> Div {
    gpui::div()
        .flex()
        .items_center()
        .justify_center()
        .px_2()
        .py_1()
        .rounded_full()
        .bg(color(status_color))
        .text_color(color(0xffffff))
        .text_xs()
        .child(label.into())
}

pub fn search_field(value: &str, placeholder: &str, colors: &ThemeColors) -> Div {
    let contents = if value.is_empty() {
        placeholder.to_string()
    } else {
        value.to_string()
    };
    let foreground = if value.is_empty() {
        colors.inactive
    } else {
        colors.fg_default
    };
    gpui::div()
        .flex()
        .items_center()
        .w_full()
        .h(px(38.))
        .px_3()
        .rounded_md()
        .border_1()
        .border_color(color(colors.border))
        .bg(color(colors.bg_surface))
        .text_color(color(foreground))
        .child(contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::LIGHT;

    #[test]
    fn visual_primitives_can_be_constructed() {
        let _ = card(&LIGHT);
        let _ = sidebar_item("服务管理", &LIGHT, true);
        let _ = stat_card("总数", 4, LIGHT.primary, &LIGHT);
        let _ = window_control("×", &LIGHT, true);
        let _ = status_badge("运行中", LIGHT.success);
    }
}
