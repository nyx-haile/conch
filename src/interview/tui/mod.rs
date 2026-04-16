pub mod keys;
pub mod state;
pub mod widgets;

pub use keys::{translate_key, UserEvent};
pub use state::{AppState, Status};

use crossterm::event::{Event, EventStream};
use futures_util::StreamExt;
use tokio::sync::mpsc;

pub async fn spawn_key_reader(tx: mpsc::Sender<UserEvent>) -> anyhow::Result<()> {
    let mut events = EventStream::new();
    while let Some(event) = events.next().await {
        match event? {
            Event::Key(k) => {
                if let Some(ev) = translate_key(&k) {
                    if tx.send(ev).await.is_err() {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
