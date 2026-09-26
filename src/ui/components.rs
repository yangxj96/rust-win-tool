//! 页面和对话框共用的 GPUI 视觉组件。
//!
//! 这些组件将与框架无关的主题值转换为 GPUI 样式，确保各页面的颜色和常见交互
//! 区域保持一致。

use gpui::prelude::*;
use gpui::{px, relative, rgb, rgba, Animation, AnimationExt, Div, FontWeight, Rgba, SharedString};
use std::time::Duration;

use crate::theme::{StatusColors, ThemeColors};

/// 将通用的 `0xRRGGBB` 颜色值转换为 GPUI 颜色。
pub fn color(value: u32) -> Rgba {
    rgb(value)
}

/// 完全透明的填充色，用于幽灵样式控件。
fn transparent() -> Rgba {
    rgba(0x00000000)
}

/// 按钮的视觉强调级别。每个页面只使用一个强调色（`Primary`）；其他按钮默认采用
/// 中性色，仅在鼠标悬停时高亮。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

/// 高度为 32px 的紧凑按钮。表格操作列等空间有限的场景可以调整返回 `Div` 的
/// 高度和内边距。
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

/// 带细边框的内容面板。层次感通过叠加背景色表达，避免使用明显阴影。
pub fn card(colors: &ThemeColors) -> Div {
    gpui::div()
        .w_full()
        .rounded_xl()
        .border_1()
        .border_color(color(colors.border.subtle))
        .bg(color(colors.bg.surface))
}

/// 导航列表项。选中项使用柔和的品牌底色和强调文字；未选中项保持低对比度，让
/// 当前页面更醒目。
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

/// 标题栏窗口控制按钮。关闭按钮悬停时显示纯红色，其他按钮使用中性悬停底色。
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

/// 柔和样式的状态徽标，包含浅色背景、易读状态文字和实心圆点。
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

/// 内嵌文本输入框。显示已提交文本、带下划线的输入法组合文本，以及聚焦时闪烁的
/// 光标。
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
        // 输入框为空时，光标位于占位文字之前的插入位置，符合常见文本框行为。
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

/// 多行文本区域。`value` 中的换行会拆成多行显示，保持脚本正文易读；聚焦时光标
/// 位于最后一行末尾。
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

/// 分组内容上方使用的小型章节说明文字。
pub fn section_label(text: impl Into<SharedString>, colors: &ThemeColors) -> Div {
    gpui::div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color(colors.fg.subtle))
        .child(text.into())
}

/// 卡片标题，视觉强调级别高于 `section_label`。
pub fn card_title(text: impl Into<SharedString>, colors: &ThemeColors) -> Div {
    gpui::div()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color(colors.fg.default))
        .child(text.into())
}

/// 绘制一组 `0..=1` 数值的柱状迷你图，所有柱子以底边对齐。
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

/// 刷新进行中显示的非确定进度条。
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

/// 居中显示的空状态，包含醒目图标、标题和提示文字。
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

/// 根据语义状态着色的行内结果提示条。
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
