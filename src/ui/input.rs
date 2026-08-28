//! Text input helpers for the GPUI frontend.
//!
//! The application has no global keyboard shortcuts. The add-service search
//! field is the only place where key events are interpreted, and this module
//! keeps that text-editing policy separate from the window.

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
