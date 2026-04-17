use crate::tts::{TextToSpeech, TtsConfig, TtsStream};
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub struct TextTts {
    tx: UnboundedSender<String>,
}

impl TextTts {
    pub fn new() -> (Self, UnboundedReceiver<String>) {
        let (tx, rx) = unbounded_channel();
        (Self { tx }, rx)
    }
}

#[async_trait]
impl TextToSpeech for TextTts {
    async fn open_stream(&self, _config: &TtsConfig) -> Result<Box<dyn TtsStream>> {
        Ok(Box::new(TextTtsStream {
            tx: self.tx.clone(),
        }))
    }

    async fn synthesize_batch(&self, texts: &[&str]) -> Result<Vec<Vec<i16>>> {
        Ok(texts.iter().map(|_| Vec::new()).collect())
    }
}

#[derive(Debug)]
pub struct TextTtsStream {
    tx: UnboundedSender<String>,
}

#[async_trait]
impl TtsStream for TextTtsStream {
    async fn push_text(&mut self, chunk: &str) -> Result<()> {
        let _ = self.tx.send(chunk.to_string());
        Ok(())
    }

    async fn end_of_input(&mut self) -> Result<()> {
        Ok(())
    }

    async fn next_chunk(&mut self) -> Option<Vec<i16>> {
        None
    }

    async fn abort(&mut self) -> Result<()> {
        Ok(())
    }

    fn sample_rate(&self) -> u32 {
        16_000
    }
}
