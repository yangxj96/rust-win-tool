//! GPUI 前端使用的文本输入辅助逻辑。
//!
//! 应用没有注册全局键盘快捷键。只有添加服务对话框中的搜索框会处理键盘事件；
//! 本模块将文本编辑规则与窗口实现分开维护。

use gpui::KeyDownEvent;

#[derive(Debug, Eq, PartialEq)]
pub enum TextEditingInput {
    Text(String),
    Backspace,
}

pub fn character(event: &KeyDownEvent) -> Option<&str> {
    if event.keystroke.modifiers.control
        || event.keystroke.modifiers.alt
        || event.keystroke.modifiers.platform
    {
        None
    } else {
        event.keystroke.key_char.as_deref()
    }
}

pub fn text_editing_input(event: &KeyDownEvent) -> Option<TextEditingInput> {
    if event.keystroke.key == "backspace" {
        Some(TextEditingInput::Backspace)
    } else {
        character(event).map(|value| TextEditingInput::Text(value.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{KeyDownEvent, Keystroke};

    fn key_event(source: &str) -> KeyDownEvent {
        KeyDownEvent {
            keystroke: Keystroke::parse(source).unwrap(),
            is_held: false,
        }
    }

    #[test]
    fn only_text_editing_input_is_interpreted() {
        assert_eq!(
            text_editing_input(&key_event("q->q")),
            Some(TextEditingInput::Text("q".to_string()))
        );
        assert_eq!(
            text_editing_input(&key_event("backspace")),
            Some(TextEditingInput::Backspace)
        );
        assert_eq!(text_editing_input(&key_event("escape")), None);
        assert_eq!(text_editing_input(&key_event("enter")), None);
        assert_eq!(text_editing_input(&key_event("down")), None);
    }
}
