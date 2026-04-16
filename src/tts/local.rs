use crate::tts::{TextToSpeech, TtsConfig, TtsStream};
use anyhow::{anyhow, Result};
use async_trait::async_trait;

pub struct LocalTts;

impl LocalTts {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalTts {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TextToSpeech for LocalTts {
    async fn open_stream(&self, _config: &TtsConfig) -> Result<Box<dyn TtsStream>> {
        Err(anyhow!("local TTS backend is not implemented yet"))
    }

    async fn synthesize_batch(&self, _texts: &[&str]) -> Result<Vec<Vec<i16>>> {
        Err(anyhow!("local TTS backend is not implemented yet"))
    }
}
