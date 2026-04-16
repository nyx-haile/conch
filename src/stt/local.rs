use crate::stt::{SpeechToText, SttConfig, SttStream};
use anyhow::{anyhow, Result};
use async_trait::async_trait;

pub struct LocalStt;

impl LocalStt {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalStt {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SpeechToText for LocalStt {
    async fn open_stream(&self, _config: &SttConfig) -> Result<Box<dyn SttStream>> {
        Err(anyhow!("local STT backend is not implemented yet"))
    }
}
