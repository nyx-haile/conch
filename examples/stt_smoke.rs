use anyhow::{Context, Result};
use conch::audio::convert::resample_i16;
use conch::config::{Config, SttBackend};
use conch::stt::{SpeechToText, SttConfig, TranscriptEvent};
use conch::tts::{TextToSpeech, TtsConfig};
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let phrase = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "hello from conch speech to text smoke".to_string());

    let home = directories::UserDirs::new()
        .context("determining home directory")?
        .home_dir()
        .to_path_buf();
    let tts = conch::tts::local::LocalTts::from_env(&home).context("opening local Piper TTS")?;
    let mut tts_stream = tts.open_stream(&TtsConfig::default()).await?;
    let sample_rate = tts_stream.sample_rate();
    tts_stream.push_text(&phrase).await?;
    tts_stream.end_of_input().await?;

    let mut pcm = Vec::new();
    while let Some(chunk) = tts_stream.next_chunk().await {
        pcm.extend(chunk);
    }
    anyhow::ensure!(!pcm.is_empty(), "local Piper produced no PCM");
    let pcm = resample_i16(&pcm, sample_rate, 16_000, 1)?;

    let config = Config::load()?;
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
    let mut stt_stream = stt
        .open_stream(&SttConfig {
            sample_rate: 16_000,
            language: Some("en-US".to_string()),
            punctuate: true,
        })
        .await?;

    for chunk in pcm.chunks(320) {
        stt_stream.send_frame(chunk).await?;
    }
    stt_stream.end_of_utterance().await?;

    let mut final_text = String::new();
    while let Ok(Some(event)) = timeout(Duration::from_secs(20), stt_stream.next_event()).await {
        if let TranscriptEvent::Final { text, .. } = event {
            if !text.is_empty() {
                final_text = text;
                break;
            }
        }
    }
    stt_stream.close().await?;

    anyhow::ensure!(!final_text.is_empty(), "STT returned no final transcript");
    println!("phrase: {phrase}");
    println!("transcript: {final_text}");
    Ok(())
}
