pub mod deepgram;
pub mod local;

use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone, Default)]
pub struct SttConfig {
    pub sample_rate: u32,
    pub language: Option<String>,
    pub punctuate: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TranscriptEvent {
    Partial { text: String, stability: f32 },
    Final { text: String, words: Vec<Word> },
    Error { message: String },
}

#[async_trait]
pub trait SpeechToText: Send + Sync {
    async fn open_stream(&self, config: &SttConfig) -> Result<Box<dyn SttStream>>;
}

#[async_trait]
pub trait SttStream: Send + std::fmt::Debug {
    async fn send_frame(&mut self, pcm: &[i16]) -> Result<()>;
    async fn end_of_utterance(&mut self) -> Result<()>;
    async fn next_event(&mut self) -> Option<TranscriptEvent>;
    async fn close(&mut self) -> Result<()>;
}
