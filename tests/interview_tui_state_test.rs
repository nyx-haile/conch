use conch::interview::tui::state::{AppState, Status, TurnView};
use conch::interview::history::Speaker;
use std::time::Duration;

#[test]
fn appstate_tracks_status_and_appends_turns() {
    let mut s = AppState::new("conch talk".into(), "# brief text".into());
    assert_eq!(s.status(), Status::Idle);

    s.set_status(Status::Listening);
    s.update_current_user_draft("hello");
    assert_eq!(s.current_user_draft(), "hello");

    s.push_turn(TurnView { speaker: Speaker::User, text: "hello world".into() });
    assert_eq!(s.history().len(), 1);
    assert_eq!(s.current_user_draft(), "");

    s.update_assistant_stream("So…");
    s.update_assistant_stream("So, tell me more.");
    assert_eq!(s.assistant_stream(), "So, tell me more.");
    s.commit_assistant();
    assert_eq!(s.history().len(), 2);
    assert_eq!(s.assistant_stream(), "");
}

#[test]
fn appstate_formats_elapsed_time() {
    let s = AppState::new("t".into(), "b".into());
    assert_eq!(s.format_elapsed(Duration::from_secs(0)), "00:00:00");
    assert_eq!(s.format_elapsed(Duration::from_secs(65)), "00:01:05");
    assert_eq!(s.format_elapsed(Duration::from_secs(3661)), "01:01:01");
}

use conch::interview::tui::keys::{translate_key, UserEvent};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn space_not_translated_here() {
    // Space is driven by a press/release state machine inside the key reader
    // (push-to-talk), not by translate_key.
    let e = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
    assert_eq!(translate_key(&e), None);
}

#[test]
fn esc_maps_to_interrupt() {
    let e = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(translate_key(&e), Some(UserEvent::Interrupt));
}

#[test]
fn ctrl_c_quits() {
    let e = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(translate_key(&e), Some(UserEvent::Quit));
}
