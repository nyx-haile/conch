pub mod deepgram;
pub mod elevenlabs;
pub mod local;
pub mod text;

use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone, Default)]
pub struct TtsConfig {
    pub voice_id: Option<String>,
}

#[async_trait]
pub trait TextToSpeech: Send + Sync {
    async fn open_stream(&self, config: &TtsConfig) -> Result<Box<dyn TtsStream>>;
    async fn synthesize_batch(&self, texts: &[&str]) -> Result<Vec<Vec<i16>>>;
}

#[async_trait]
pub trait TtsStream: Send + std::fmt::Debug {
    async fn push_text(&mut self, chunk: &str) -> Result<()>;
    async fn end_of_input(&mut self) -> Result<()>;
    async fn next_chunk(&mut self) -> Option<Vec<i16>>;
    async fn abort(&mut self) -> Result<()>;
    /// Sample rate of the PCM returned by `next_chunk`. Backends know this
    /// from their voice config (piper), stream output format (ElevenLabs),
    /// or trivially (text mode).
    fn sample_rate(&self) -> u32;
}
