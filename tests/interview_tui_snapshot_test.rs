use conch::interview::history::Speaker;
use conch::interview::tui::state::{AppState, Status, TurnView};
use conch::interview::tui::widgets::render_frame;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::time::Duration;

fn make_state() -> AppState {
    let mut s = AppState::new("conch talk".into(), "# Brief\n\n## Summary\nTest.".into());
    s.push_turn(TurnView {
        speaker: Speaker::Conch,
        text: "So, what made you build this?".into(),
    });
    s.push_turn(TurnView {
        speaker: Speaker::User,
        text: "Wanted to learn audio.".into(),
    });
    s
}

#[test]
fn renders_listening_state_without_panic() {
    let backend = TestBackend::new(100, 30);
    let mut term = Terminal::new(backend).unwrap();
    let mut st = make_state();
    st.set_status(Status::Listening);
    st.update_current_user_draft("and also I wanted to");
    st.set_waveform(vec![0.1, 0.3, 0.6, 0.9, 0.6, 0.3, 0.1]);
    term.draw(|f| render_frame(f, &st, Duration::from_secs(42)))
        .unwrap();
    let buf = term.backend().buffer().clone();
    let dump = buffer_text(&buf);
    assert!(
        dump.contains("Listening"),
        "expected 'Listening' in:\n{dump}"
    );
    assert!(dump.contains("00:00:42"), "expected '00:00:42' in:\n{dump}");
    assert!(
        dump.contains("conch talk"),
        "expected 'conch talk' in:\n{dump}"
    );
    assert!(dump.contains("Brief"), "expected 'Brief' in:\n{dump}");
}

#[test]
fn renders_thinking_and_speaking_states() {
    for status in [
        Status::Thinking,
        Status::Filling,
        Status::Speaking,
        Status::Filler,
    ] {
        let backend = TestBackend::new(100, 30);
        let mut term = Terminal::new(backend).unwrap();
        let mut st = make_state();
        st.set_status(status);
        st.update_assistant_stream("Interesting, let me think");
        term.draw(|f| render_frame(f, &st, Duration::from_secs(1)))
            .unwrap();
    }
}

fn buffer_text(buf: &ratatui::buffer::Buffer) -> String {
    let area = buf.area;
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            out.push_str(buf.get(x, y).symbol());
        }
        out.push('\n');
    }
    out
}
