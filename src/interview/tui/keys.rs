// src/interview/tui/keys.rs
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserEvent {
    MicToggle,
    Interrupt,
    Quit,
}

/// Translate a non-space key event. Space-bar handling lives in the key reader
/// because it drives a press/release state machine (push-to-talk).
pub fn translate_key(event: &KeyEvent) -> Option<UserEvent> {
    if event.kind != KeyEventKind::Press {
        return None;
    }
    match (event.code, event.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(UserEvent::Quit),
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
    fn space_not_translated_here() {
        assert_eq!(
            translate_key(&key(KeyCode::Char(' '), KeyEventKind::Press)),
            None
        );
    }

    #[test]
    fn ctrl_c_yields_quit() {
        let k = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        assert_eq!(translate_key(&k), Some(UserEvent::Quit));
    }

    #[test]
    fn esc_yields_interrupt() {
        assert_eq!(
            translate_key(&key(KeyCode::Esc, KeyEventKind::Press)),
            Some(UserEvent::Interrupt)
        );
    }

    #[test]
    fn release_ignored() {
        let k = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        };
        assert_eq!(translate_key(&k), None);
    }
}
