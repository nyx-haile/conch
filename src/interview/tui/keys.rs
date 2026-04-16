// src/interview/tui/keys.rs
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserEvent {
    MicToggle,
    Interrupt,
    Quit,
}

pub fn translate_key(event: &KeyEvent) -> Option<UserEvent> {
    match (event.code, event.modifiers) {
        (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => Some(UserEvent::Quit),
        (KeyCode::Char(' '), _) => Some(UserEvent::MicToggle),
        (KeyCode::Esc, _) => Some(UserEvent::Interrupt),
        _ => None,
    }
}
