//! Pipeline check: run piper end-to-end, print sample count and rate. No
//! audio device needed — useful for sandboxed or CI environments.

use conch::tts::{TextToSpeech, TtsConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let home = directories::UserDirs::new()
        .unwrap()
        .home_dir()
        .to_path_buf();
    let tts = conch::tts::local::LocalTts::from_env(&home)?;
    let mut stream = tts.open_stream(&TtsConfig::default()).await?;
    let rate = stream.sample_rate();
    println!("sample_rate: {rate}");
    stream
        .push_text("Hello world from the pipeline check.")
        .await?;
    stream.end_of_input().await?;
    let mut total = 0usize;
    while let Some(pcm) = stream.next_chunk().await {
        total += pcm.len();
    }
    let secs = total as f32 / rate as f32;
    println!("samples: {total} ({secs:.2}s @ {rate} Hz)");
    Ok(())
}
