use anyhow::{Context, Result};
use chrono::Local;
use conch::audio::input::{CpalMicSource, Frame, MicGate};
use conch::audio::wav::WavSessionWriter;
use conch::config::{Config, SttBackend};
use conch::session::Session;
use conch::stt::{SpeechToText, SttConfig, TranscriptEvent};
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let seconds = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(6);

    let config = Config::load()?;
    let date = Local::now().format("%Y-%m-%d").to_string();
    let session = Session::create(&config.sessions_dir(), &date, "dictation-smoke")?;
    let mut stt = open_stt(&config).await?;
    let source = CpalMicSource::new_default(20).context("opening mic")?;
    let writer = WavSessionWriter::create(&session.raw_audio_path(), 16_000, 1)
        .context("creating raw mic audio recorder")?;
    let (tx, mut rx) = broadcast::channel::<Frame>(256);
    let mut gate = MicGate::new(Box::new(source), tx).with_recorder(writer);
    gate.set_open(true);
    let gate_task = tokio::spawn(async move { gate.run().await });

    let mut frames = 0usize;
    let mut peak_rms = 0.0f32;
    let deadline = Instant::now() + Duration::from_secs(seconds);
    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        match timeout(remaining, rx.recv()).await {
            Ok(Ok(frame)) => {
                peak_rms = peak_rms.max(frame_rms(&frame.pcm));
                stt.send_frame(&frame.pcm).await?;
                frames += 1;
            }
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(broadcast::error::RecvError::Closed)) | Err(_) => break,
        }
    }
    gate_task.abort();
    let _ = gate_task.await;

    stt.end_of_utterance().await?;
    let transcript = drain_final_text(&mut *stt, Duration::from_secs(30)).await;
    stt.close().await?;

    let bytes = std::fs::metadata(session.raw_audio_path())?.len();
    anyhow::ensure!(frames > 0, "mic produced no frames");
    anyhow::ensure!(bytes > 44, "raw audio wav contains no samples");
    anyhow::ensure!(peak_rms > 0.001, "captured mic audio appears silent");
    anyhow::ensure!(!transcript.is_empty(), "STT returned no final transcript");

    println!("session: {}", session.id().as_str());
    println!("raw_audio: {}", session.raw_audio_path().display());
    println!("bytes: {bytes}");
    println!("frames: {frames}");
    println!("peak_rms: {peak_rms:.4}");
    println!("transcript: {transcript}");
    Ok(())
}

async fn open_stt(config: &Config) -> Result<Box<dyn conch::stt::SttStream>> {
    let stt: Box<dyn SpeechToText> = match config.stt_backend() {
        SttBackend::Deepgram => Box::new(conch::stt::deepgram::DeepgramStt::production(
            config
                .deepgram_api_key()
                .context("DEEPGRAM_API_KEY required")?,
        )),
        SttBackend::Local => {
            let stt = conch::stt::local::LocalStt::new(config.parakeet_model_dir());
            stt.prepare().await?;
            Box::new(stt)
        }
    };
    stt.open_stream(&SttConfig {
        sample_rate: 16_000,
        language: Some("en-US".to_string()),
        punctuate: true,
    })
    .await
}

async fn drain_final_text(stream: &mut dyn conch::stt::SttStream, duration: Duration) -> String {
    let deadline = Instant::now() + duration;
    let mut finals = Vec::new();
    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        match timeout(remaining, stream.next_event()).await {
            Ok(Some(TranscriptEvent::Final { text, .. })) if !text.is_empty() => finals.push(text),
            Ok(Some(_)) => continue,
            Ok(None) | Err(_) => break,
        }
    }
    finals.join(" ")
}

fn frame_rms(pcm: &[i16]) -> f32 {
    if pcm.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = pcm
        .iter()
        .map(|&sample| {
            let sample = sample as f64 / i16::MAX as f64;
            sample * sample
        })
        .sum();
    (sum_sq / pcm.len() as f64).sqrt() as f32
}
