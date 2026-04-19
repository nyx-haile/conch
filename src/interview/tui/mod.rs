pub mod keys;
pub mod state;
pub mod widgets;

pub use keys::{translate_key, UserEvent};
pub use state::{AppState, Status};

use crossterm::event::{Event, EventStream};
use futures_util::StreamExt;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Minimum gap between accepted MicToggle events. On terminals without the
/// Kitty keyboard protocol, OS auto-repeat shows up as a stream of Press
/// events at ~30ms cadence; this gap keeps a held space key from spamming
/// toggles. A genuine release-then-repress takes longer than this.
const MIC_TOGGLE_DEBOUNCE: Duration = Duration::from_millis(150);

pub async fn spawn_key_reader(tx: mpsc::Sender<UserEvent>) -> anyhow::Result<()> {
    let mut events = EventStream::new();
    let mut last_mic_toggle: Option<Instant> = None;
    while let Some(event) = events.next().await {
        if let Event::Key(k) = event? {
            if let Some(ev) = translate_key(&k) {
                if ev == UserEvent::MicToggle {
                    let now = Instant::now();
                    let held = last_mic_toggle
                        .is_some_and(|prev| now.duration_since(prev) < MIC_TOGGLE_DEBOUNCE);
                    // Refresh the timestamp on every Press, accepted or not.
                    // This way a held key (auto-repeat ~30ms) keeps extending
                    // the window and never re-fires; only a real release for
                    // >= MIC_TOGGLE_DEBOUNCE allows the next toggle through.
                    last_mic_toggle = Some(now);
                    if held {
                        continue;
                    }
                }
                if tx.send(ev).await.is_err() {
                    break;
                }
            }
        }
    }
    Ok(())
}
