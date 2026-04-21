pub mod keys;
pub mod state;
pub mod widgets;

pub use keys::{translate_key, UserEvent};
pub use state::{AppState, Status};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Press → release under this duration is treated as a tap (toggle semantics);
/// anything longer is a hold (release ends recording after a backoff).
const HOLD_THRESHOLD: Duration = Duration::from_millis(300);

/// After a hold release, wait this long before actually stopping the mic.
/// A fresh press inside this window cancels the pending stop so a brief
/// finger-lift in the middle of speaking does not clip the utterance.
const RELEASE_BACKOFF: Duration = Duration::from_millis(200);

/// Fallback (terminals without kitty keyboard protocol): minimum gap between
/// accepted MicToggle events. OS auto-repeat shows up as a Press stream at
/// ~30ms cadence; this gap keeps a held space key from spamming toggles.
const LEGACY_DEBOUNCE: Duration = Duration::from_millis(150);

pub async fn spawn_key_reader(
    tx: mpsc::Sender<UserEvent>,
    hold_mode: bool,
) -> anyhow::Result<()> {
    if hold_mode {
        spawn_hold_reader(tx).await
    } else {
        spawn_legacy_reader(tx).await
    }
}

// ---------------------------------------------------------------------------
// Legacy reader: press-only, debounced toggle. Used on terminals without the
// kitty keyboard protocol, where KeyEventKind::Release never arrives.
// ---------------------------------------------------------------------------

async fn spawn_legacy_reader(tx: mpsc::Sender<UserEvent>) -> anyhow::Result<()> {
    let mut events = EventStream::new();
    let mut last_mic_toggle: Option<Instant> = None;
    while let Some(event) = events.next().await {
        let Event::Key(k) = event? else { continue };
        if k.kind != KeyEventKind::Press {
            continue;
        }
        if matches!(k.code, KeyCode::Char(' ')) {
            let now = Instant::now();
            let held = last_mic_toggle
                .is_some_and(|prev| now.duration_since(prev) < LEGACY_DEBOUNCE);
            last_mic_toggle = Some(now);
            if held {
                continue;
            }
            if tx.send(UserEvent::MicToggle).await.is_err() {
                break;
            }
            continue;
        }
        if let Some(ev) = translate_key(&k) {
            if tx.send(ev).await.is_err() {
                break;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Hold reader: distinguishes tap (toggle) from hold (push-to-talk with
// release-debounce). Requires KeyEventKind::Release from the terminal.
// ---------------------------------------------------------------------------

async fn spawn_hold_reader(tx: mpsc::Sender<UserEvent>) -> anyhow::Result<()> {
    let mut events = EventStream::new();
    let mut fsm = HoldFsm::default();

    loop {
        let deadline = fsm.next_deadline();
        tokio::select! {
            biased;
            event = events.next() => {
                let Some(ev) = event else { break };
                let Event::Key(k) = ev? else { continue };

                if !matches!(k.code, KeyCode::Char(' ')) {
                    if let Some(ev) = translate_key(&k) {
                        if tx.send(ev).await.is_err() {
                            break;
                        }
                    }
                    continue;
                }

                let edge = match k.kind {
                    KeyEventKind::Press => fsm.on_press(Instant::now()),
                    KeyEventKind::Release => {
                        fsm.on_release(Instant::now(), HOLD_THRESHOLD, RELEASE_BACKOFF)
                    }
                    KeyEventKind::Repeat => None,
                };
                if edge.is_some() && tx.send(UserEvent::MicToggle).await.is_err() {
                    break;
                }
            }
            _ = async {
                match deadline {
                    Some(d) => tokio::time::sleep_until(d.into()).await,
                    None => std::future::pending::<()>().await,
                }
            } => {
                if fsm.tick(Instant::now()).is_some()
                    && tx.send(UserEvent::MicToggle).await.is_err()
                {
                    break;
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// HoldFsm — pure state machine (testable)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
enum MicEdge {
    On,
    Off,
}

#[derive(Default)]
struct HoldFsm {
    recording: bool,
    /// Set while space is physically held down; cleared on release.
    press_at: Option<Instant>,
    /// When Some, a backoff stop is scheduled. A fresh press before this
    /// instant cancels it.
    stop_deadline: Option<Instant>,
}

impl HoldFsm {
    fn on_press(&mut self, now: Instant) -> Option<MicEdge> {
        if self.stop_deadline.is_some() {
            // Re-press inside the release-backoff window — cancel the pending
            // stop and keep recording.
            self.stop_deadline = None;
            self.press_at = Some(now);
            return None;
        }
        if !self.recording {
            self.recording = true;
            self.press_at = Some(now);
            return Some(MicEdge::On);
        }
        // Recording but key is not physically held: the previous press was a
        // tap that left the mic on. This press closes it (tap-toggle).
        if self.press_at.is_none() {
            self.recording = false;
            return Some(MicEdge::Off);
        }
        // Already recording and key physically held — nothing to do.
        None
    }

    fn on_release(
        &mut self,
        now: Instant,
        hold_threshold: Duration,
        backoff: Duration,
    ) -> Option<MicEdge> {
        let Some(at) = self.press_at.take() else {
            return None;
        };
        let held = now.duration_since(at);
        if held >= hold_threshold {
            self.stop_deadline = Some(now + backoff);
        }
        // Else: tap. Keep recording on; wait for next press to toggle off.
        None
    }

    fn tick(&mut self, now: Instant) -> Option<MicEdge> {
        let d = self.stop_deadline?;
        if now < d {
            return None;
        }
        self.stop_deadline = None;
        if self.recording {
            self.recording = false;
            Some(MicEdge::Off)
        } else {
            None
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        self.stop_deadline
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HT: Duration = Duration::from_millis(300);
    const BO: Duration = Duration::from_millis(200);

    #[test]
    fn tap_press_release_does_not_stop() {
        let mut fsm = HoldFsm::default();
        let t0 = Instant::now();
        assert_eq!(fsm.on_press(t0), Some(MicEdge::On));
        let release = t0 + Duration::from_millis(80);
        assert_eq!(fsm.on_release(release, HT, BO), None);
        assert!(fsm.recording);
        assert!(fsm.next_deadline().is_none());
    }

    #[test]
    fn tap_then_tap_toggles_off() {
        let mut fsm = HoldFsm::default();
        let t0 = Instant::now();
        fsm.on_press(t0);
        fsm.on_release(t0 + Duration::from_millis(50), HT, BO);
        let t1 = t0 + Duration::from_millis(1_000);
        assert_eq!(fsm.on_press(t1), Some(MicEdge::Off));
        assert!(!fsm.recording);
    }

    #[test]
    fn hold_release_schedules_stop() {
        let mut fsm = HoldFsm::default();
        let t0 = Instant::now();
        fsm.on_press(t0);
        let release = t0 + Duration::from_millis(600);
        assert_eq!(fsm.on_release(release, HT, BO), None);
        assert!(fsm.recording);
        assert_eq!(fsm.next_deadline(), Some(release + BO));
    }

    #[test]
    fn hold_stop_fires_after_backoff() {
        let mut fsm = HoldFsm::default();
        let t0 = Instant::now();
        fsm.on_press(t0);
        let release = t0 + Duration::from_millis(600);
        fsm.on_release(release, HT, BO);
        assert_eq!(fsm.tick(release + Duration::from_millis(100)), None);
        assert!(fsm.recording);
        assert_eq!(fsm.tick(release + BO), Some(MicEdge::Off));
        assert!(!fsm.recording);
        assert!(fsm.next_deadline().is_none());
    }

    #[test]
    fn repress_inside_backoff_cancels_stop() {
        let mut fsm = HoldFsm::default();
        let t0 = Instant::now();
        fsm.on_press(t0);
        let release = t0 + Duration::from_millis(600);
        fsm.on_release(release, HT, BO);
        let repress = release + Duration::from_millis(100);
        assert_eq!(fsm.on_press(repress), None);
        assert!(fsm.recording);
        assert!(fsm.next_deadline().is_none());
        // Stop no longer fires.
        assert_eq!(fsm.tick(release + BO + Duration::from_millis(50)), None);
        assert!(fsm.recording);
    }

    #[test]
    fn repress_after_backoff_elapsed_but_before_tick_still_cancels() {
        // Edge: the deadline passed but we haven't ticked yet; a press should
        // still cancel the pending stop rather than re-arming.
        let mut fsm = HoldFsm::default();
        let t0 = Instant::now();
        fsm.on_press(t0);
        let release = t0 + Duration::from_millis(600);
        fsm.on_release(release, HT, BO);
        let repress = release + BO + Duration::from_millis(10);
        assert_eq!(fsm.on_press(repress), None);
        assert!(fsm.recording);
    }
}
