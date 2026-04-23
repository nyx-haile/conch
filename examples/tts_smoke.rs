//! Smoke-test a TTS backend end-to-end: open a stream, push a phrase,
//! decode chunks, and play them through the default audio output.
//!
//! Backend is chosen by CONCH_TTS (same as the CLI):
//!   CONCH_TTS=local       → piper (default)
//!   CONCH_TTS=elevenlabs  → ElevenLabs (needs ELEVENLABS_API_KEY)
//!
//! Usage:
//!   cargo run --example tts_smoke
//!   cargo run --example tts_smoke -- "custom phrase"
//!   CONCH_TTS=elevenlabs ELEVENLABS_API_KEY=sk_... cargo run --example tts_smoke
//!   CONCH_OUTPUT_DEVICE=pulse cargo run --example tts_smoke

use anyhow::{Context, Result};
use conch::audio::output::{AudioSink, RodioSink};
use conch::tts::{TextToSpeech, TtsConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let backend = std::env::var("CONCH_TTS").unwrap_or_else(|_| "local".to_string());
    let phrase = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Hello from the conch smoke test.".to_string());

    println!("backend: {backend}");
    println!("phrase:  {phrase}");

    let tts: Box<dyn TextToSpeech> = match backend.as_str() {
        "local" => {
            let home = directories::UserDirs::new()
                .context("determining home directory")?
                .home_dir()
                .to_path_buf();
            Box::new(conch::tts::local::LocalTts::from_env(&home)?)
        }
        "elevenlabs" => {
            let key = std::env::var("ELEVENLABS_API_KEY").map_err(|_| {
                anyhow::anyhow!("ELEVENLABS_API_KEY not set. Export it or use CONCH_TTS=local.")
            })?;
            let voice_id = std::env::var("CONCH_ELEVEN_VOICE_ID")
                .unwrap_or_else(|_| "21m00Tcm4TlvDq8ikWAM".to_string());
            Box::new(conch::tts::elevenlabs::ElevenLabsTts::production(
                key, voice_id,
            ))
        }
        other => anyhow::bail!("unknown CONCH_TTS backend {other:?}; use local or elevenlabs"),
    };

    println!("opening stream...");
    let mut stream = tts
        .open_stream(&TtsConfig::default())
        .await
        .context("opening TTS stream")?;
    let rate = stream.sample_rate();
    println!("stream sample rate: {rate} Hz");

    stream.push_text(&phrase).await?;
    stream.end_of_input().await?;

    let mut sink = RodioSink::new_default().context("opening audio output")?;
    let mut total: usize = 0;
    while let Some(pcm) = stream.next_chunk().await {
        total += pcm.len();
        sink.push(pcm, rate)?;
    }

    let secs = total as f32 / rate as f32;
    println!("received {total} samples (~{secs:.2}s); draining playback...");
    // Sleep through the audio duration plus a tail, then drop the sink so
    // its Drop sends Shutdown and joins the worker. Do NOT call sink.stop()
    // — that clears the queue and cuts off any unplayed audio.
    let drain_ms = (secs * 1000.0) as u64 + 1_500;
    tokio::time::sleep(std::time::Duration::from_millis(drain_ms)).await;
    drop(sink);
    println!("done.");
    Ok(())
}
