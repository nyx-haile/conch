// src/interview/tui/keys.rs
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserEvent {
    MicToggle,
    Interrupt,
    Quit,
}

pub fn translate_key(event: &KeyEvent) -> Option<UserEvent> {
    // Ignore Repeat/Release. On terminals with the Kitty keyboard protocol
    // enabled, OS auto-repeat surfaces as KeyEventKind::Repeat; filtering it
    // here keeps a held key from re-toggling the mic.
    if event.kind != KeyEventKind::Press {
        return None;
    }
    match (event.code, event.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(UserEvent::Quit),
        (KeyCode::Char(' '), _) => Some(UserEvent::MicToggle),
        (KeyCode::Esc, _) => Some(UserEvent::Interrupt),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};

    fn key(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn press_space_yields_mic_toggle() {
        assert_eq!(
            translate_key(&key(KeyCode::Char(' '), KeyEventKind::Press)),
            Some(UserEvent::MicToggle)
        );
    }

    #[test]
    fn repeat_space_ignored() {
        assert_eq!(
            translate_key(&key(KeyCode::Char(' '), KeyEventKind::Repeat)),
            None
        );
    }

    #[test]
    fn release_space_ignored() {
        assert_eq!(
            translate_key(&key(KeyCode::Char(' '), KeyEventKind::Release)),
            None
        );
    }
}
