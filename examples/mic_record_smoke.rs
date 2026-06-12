use anyhow::{Context, Result};
use chrono::Local;
use conch::audio::input::{CpalMicSource, MicGate};
use conch::audio::wav::WavSessionWriter;
use conch::config::Config;
use conch::session::Session;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let seconds = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(3);
    let config = Config::load()?;
    let date = Local::now().format("%Y-%m-%d").to_string();
    let session = Session::create(&config.sessions_dir(), &date, "mic-record-smoke")?;
    let writer = WavSessionWriter::create(&session.raw_audio_path(), 16_000, 1)
        .context("creating raw mic audio recorder")?;
    let source = CpalMicSource::new_default(20).context("opening default mic")?;
    let (tx, _rx) = broadcast::channel(256);
    let mut gate = MicGate::new(Box::new(source), tx).with_recorder(writer);
    gate.set_open(true);
    let handle = tokio::spawn(async move { gate.run().await });
    sleep(Duration::from_secs(seconds)).await;
    handle.abort();
    let _ = handle.await;
    let bytes = std::fs::metadata(session.raw_audio_path())?.len();
    anyhow::ensure!(bytes > 44, "raw audio wav contains no samples");
    println!("session: {}", session.id().as_str());
    println!("raw_audio: {}", session.raw_audio_path().display());
    println!("bytes: {bytes}");
    Ok(())
}
