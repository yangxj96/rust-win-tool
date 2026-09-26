//! Shared GPUI visual primitives used by pages and dialogs.
//!
//! Components translate framework-neutral theme tokens into GPUI styling so
//! colors and common interaction surfaces stay consistent across pages.

use gpui::prelude::*;
use gpui::{px, relative, rgb, rgba, Animation, AnimationExt, Div, FontWeight, Rgba, SharedString};
use std::time::Duration;

use crate::theme::{StatusColors, ThemeColors};

/// Convert a framework-neutral `0xRRGGBB` token into a GPUI color.
pub fn color(value: u32) -> Rgba {
    rgb(value)
}

/// A fully transparent fill, used by ghost controls.
fn transparent() -> Rgba {
    rgba(0x00000000)
}

/// Visual weight of a button. There is exactly one accent (`Primary`) per
/// screen; everything else is neutral until the user hovers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

/// Compact, 32px-tall button. Callers may override height and padding on the
/// returned `Div` for dense contexts such as table action columns.
pub fn button(label: impl Into<SharedString>, colors: &ThemeColors, variant: ButtonVariant) -> Div {
    let base = gpui::div()
        .flex()
        .items_center()
        .justify_center()
        .h(px(32.))
        .px_3()
        .rounded_lg()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .cursor_pointer();

    let styled = match variant {
        ButtonVariant::Primary => base
            .bg(color(colors.brand.fill))
            .text_color(color(colors.fg.on_accent))
            .hover(move |style| style.bg(color(colors.brand.fill_hover))),
        ButtonVariant::Secondary => base
            .bg(color(colors.bg.surface))
            .text_color(color(colors.fg.default))
            .border_1()
            .border_color(color(colors.border.default))
            .hover(move |style| style.bg(color(colors.bg.surface_hover))),
        ButtonVariant::Ghost => base
            .bg(transparent())
            .text_color(color(colors.fg.muted))
            .hover(move |style| {
                style
                    .bg(color(colors.bg.surface_hover))
                    .text_color(color(colors.fg.default))
            }),
        ButtonVariant::Danger => base
            .bg(transparent())
            .text_color(color(colors.danger.text))
            .hover(move |style| {
                style
                    .bg(color(colors.danger.soft))
                    .text_color(color(colors.danger.text))
            }),
    };

    styled.child(label.into())
}

/// A surface panel with a hairline border. Elevation is expressed through the
/// layered background tokens rather than heavy shadows.
pub fn card(colors: &ThemeColors) -> Div {
    gpui::div()
        .w_full()
        .rounded_xl()
        .border_1()
        .border_color(color(colors.border.subtle))
        .bg(color(colors.bg.surface))
}

/// A navigation row. Selected rows use the soft brand tint plus the accent
/// text color; unselected rows stay muted so the active page stands out.
pub fn sidebar_item(
    icon: &str,
    label: impl Into<SharedString>,
    colors: &ThemeColors,
    selected: bool,
) -> Div {
    let base = gpui::div()
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .h(px(40.))
        .px_3()
        .rounded_lg()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer();

    let base = if selected {
        base.bg(color(colors.brand.soft))
            .text_color(color(colors.brand.primary))
    } else {
        base.bg(transparent()).text_color(color(colors.fg.muted))
    };

    base.hover(move |style| style.bg(color(colors.bg.surface_hover)))
        .child(
            gpui::div()
                .w(px(16.))
                .text_center()
                .text_base()
                .child(icon.to_string()),
        )
        .child(label.into())
}

/// Window titlebar control. The close control turns solid red on hover while
/// the others fall back to the neutral hover surface.
pub fn window_control(label: impl Into<SharedString>, colors: &ThemeColors, danger: bool) -> Div {
    let hover_background = if danger {
        colors.danger.solid
    } else {
        colors.bg.surface_hover
    };
    let hover_foreground = if danger {
        colors.fg.on_accent
    } else {
        colors.fg.default
    };
    gpui::div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(44.))
        .h(px(40.))
        .text_color(color(colors.fg.muted))
        .cursor_pointer()
        .hover(move |style| {
            style
                .bg(color(hover_background))
                .text_color(color(hover_foreground))
        })
        .child(label.into())
}

/// Soft status pill: tinted background, readable status text and a solid dot.
pub fn badge(label: impl Into<SharedString>, status: &StatusColors) -> Div {
    gpui::div()
        .flex()
        .items_center()
        .gap_1()
        .h(px(22.))
        .px_2()
        .rounded_full()
        .bg(color(status.soft))
        .text_color(color(status.text))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .flex_none()
        .whitespace_nowrap()
        .child(
            gpui::div()
                .w(px(6.))
                .h(px(6.))
                .rounded_full()
                .bg(color(status.solid)),
        )
        .child(label.into())
}

/// Inset text field. Shows the committed value, any in-progress IME
/// composition (underlined) and a blinking caret while focused.
pub fn search_field(
    value: &str,
    marked: &str,
    placeholder: &str,
    colors: &ThemeColors,
    focused: bool,
) -> Div {
    let mut contents = gpui::div()
        .flex()
        .items_center()
        .overflow_hidden()
        .whitespace_nowrap();

    let caret = |colors: &ThemeColors| {
        gpui::div()
            .w(px(1.5))
            .h(px(16.))
            .ml(px(1.))
            .bg(color(colors.fg.default))
            .with_animation(
                "search-caret",
                Animation::new(Duration::from_millis(1060)).repeat(),
                |element, delta| element.opacity(if delta < 0.5 { 1.0 } else { 0.0 }),
            )
    };

    if value.is_empty() && marked.is_empty() {
        // Empty field: the caret sits at the insertion point, before the
        // placeholder, matching the usual text-field behaviour.
        if focused {
            contents = contents.child(caret(colors));
        }
        contents = contents.child(
            gpui::div()
                .text_color(color(colors.fg.subtle))
                .child(placeholder.to_string()),
        );
    } else {
        contents = contents.child(
            gpui::div()
                .text_color(color(colors.fg.default))
                .child(value.to_string()),
        );
        if !marked.is_empty() {
            contents = contents.child(
                gpui::div()
                    .text_color(color(colors.brand.primary))
                    .underline()
                    .child(marked.to_string()),
            );
        }
        if focused {
            contents = contents.child(caret(colors));
        }
    }

    gpui::div()
        .flex()
        .items_center()
        .w_full()
        .h(px(34.))
        .px_3()
        .rounded_lg()
        .border_1()
        .border_color(color(colors.border.strong))
        .bg(color(colors.bg.canvas))
        .child(contents)
}

/// Multi-line text area. Line breaks in `value` are rendered as separate rows
/// so wrapped bodies stay readable and the caret sits at the end of the last
/// line while focused.
pub fn text_area(
    value: &str,
    marked: &str,
    placeholder: &str,
    colors: &ThemeColors,
    focused: bool,
    height: f32,
) -> Div {
    let caret = || {
        gpui::div()
            .w(px(1.5))
            .h(px(16.))
            .ml(px(1.))
            .bg(color(colors.fg.default))
            .with_animation(
                "text-area-caret",
                Animation::new(Duration::from_millis(1060)).repeat(),
                |element, delta| element.opacity(if delta < 0.5 { 1.0 } else { 0.0 }),
            )
    };

    let mut body = gpui::div().flex().flex_col().w_full();

    if value.is_empty() && marked.is_empty() {
        let mut line = gpui::div().flex().items_center().min_h(px(18.));
        if focused {
            line = line.child(caret());
        }
        line = line.child(
            gpui::div()
                .text_color(color(colors.fg.subtle))
                .child(placeholder.to_string()),
        );
        body = body.child(line);
    } else {
        let parts: Vec<&str> = value.split('\n').collect();
        let last = parts.len().saturating_sub(1);
        for (index, part) in parts.iter().enumerate() {
            let is_last = index == last;
            let mut line = gpui::div().flex().items_center().w_full().min_h(px(18.));
            if part.is_empty() {
                line = line.child(gpui::div().child(" "));
            } else {
                line = line.child(
                    gpui::div()
                        .flex_1()
                        .min_w(px(0.))
                        .whitespace_normal()
                        .text_color(color(colors.fg.default))
                        .child((*part).to_string()),
                );
            }
            if is_last {
                if !marked.is_empty() {
                    line = line.child(
                        gpui::div()
                            .text_color(color(colors.brand.primary))
                            .underline()
                            .child(marked.to_string()),
                    );
                }
                if focused {
                    line = line.child(caret());
                }
            }
            body = body.child(line);
        }
    }

    gpui::div()
        .w_full()
        .h(px(height))
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(color(colors.border.strong))
        .bg(color(colors.bg.canvas))
        .overflow_hidden()
        .child(body)
}

/// Small uppercase-ish section caption used above grouped content.
pub fn section_label(text: impl Into<SharedString>, colors: &ThemeColors) -> Div {
    gpui::div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color(colors.fg.subtle))
        .child(text.into())
}

/// Card heading, one step stronger than `section_label`.
pub fn card_title(text: impl Into<SharedString>, colors: &ThemeColors) -> Div {
    gpui::div()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color(colors.fg.default))
        .child(text.into())
}

/// Bar sparkline for a series of 0..=1 values, anchored to the bottom.
pub fn sparkline(values: &[f32], accent: u32) -> Div {
    gpui::div()
        .flex()
        .items_end()
        .gap(px(1.))
        .h(px(48.))
        .w_full()
        .children(values.iter().map(|&value| {
            gpui::div()
                .flex_1()
                .rounded_t_sm()
                .bg(color(accent))
                .h(relative(value.clamp(0.02, 1.0)))
        }))
}

/// Indeterminate progress bar shown while a refresh is in flight.
pub fn progress_bar(colors: &ThemeColors) -> Div {
    gpui::div()
        .w_full()
        .h(px(2.))
        .rounded_full()
        .overflow_hidden()
        .bg(color(colors.bg.muted))
        .child(
            gpui::div()
                .h_full()
                .rounded_full()
                .bg(color(colors.brand.primary))
                .with_animation(
                    "refresh-progress",
                    Animation::new(Duration::from_millis(900)).repeat(),
                    |element, delta| element.w(relative(0.35)).ml(relative(delta * 0.65)),
                ),
        )
}

/// Centered empty state with a large glyph, a title and a hint line.
pub fn empty_state(
    icon: &str,
    title: impl Into<SharedString>,
    hint: impl Into<SharedString>,
    colors: &ThemeColors,
) -> Div {
    gpui::div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .child(
            gpui::div()
                .text_3xl()
                .text_color(color(colors.fg.subtle))
                .child(icon.to_string()),
        )
        .child(
            gpui::div()
                .text_base()
                .font_weight(FontWeight::MEDIUM)
                .text_color(color(colors.fg.default))
                .child(title.into()),
        )
        .child(
            gpui::div()
                .text_sm()
                .text_color(color(colors.fg.muted))
                .text_center()
                .child(hint.into()),
        )
}

/// Inline result banner tinted by a semantic status.
pub fn alert(text: impl Into<SharedString>, status: &StatusColors) -> Div {
    gpui::div()
        .flex()
        .items_start()
        .gap_2()
        .w_full()
        .p_3()
        .rounded_lg()
        .bg(color(status.soft))
        .text_color(color(status.text))
        .text_sm()
        .child(
            gpui::div()
                .mt(px(6.))
                .w(px(6.))
                .h(px(6.))
                .rounded_full()
                .bg(color(status.solid)),
        )
        .child(gpui::div().flex_1().child(text.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::LIGHT;

    #[test]
    fn visual_primitives_can_be_constructed() {
        let _ = card(&LIGHT);
        let _ = sidebar_item("▤", "服务管理", &LIGHT, true);
        let _ = card_title("系统", &LIGHT);
        let _ = progress_bar(&LIGHT);
        let _ = window_control("×", &LIGHT, true);
        let _ = badge("运行中", &LIGHT.success);
        let _ = search_field("", "", "搜索", &LIGHT, true);
        let _ = section_label("导航", &LIGHT);
        let _ = empty_state("▤", "暂无", "点击添加", &LIGHT);
        let _ = alert("完成", &LIGHT.success);
        let _ = button("确定", &LIGHT, ButtonVariant::Primary);
    }
}
