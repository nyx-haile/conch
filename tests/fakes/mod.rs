use anyhow::Result;
use async_trait::async_trait;
use conch::audio::output::AudioSink;
use conch::interview::orchestrator::LlmCaller;
use conch::llm::types::{
    ChatRequest, ChatResponse, Choice, Message, Role,
};
use conch::stt::{SpeechToText, SttConfig, SttStream, TranscriptEvent};
use conch::tts::{TextToSpeech, TtsConfig, TtsStream};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use conch::audio::input::Frame;

// ---------------------------------------------------------------------------
// FakeSink — collects PCM samples, asserts playback happened
// ---------------------------------------------------------------------------

pub struct FakeSink {
    collected: Arc<Mutex<Vec<i16>>>,
}

impl FakeSink {
    pub fn new() -> Self {
        Self {
            collected: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn collected(&self) -> Arc<Mutex<Vec<i16>>> {
        self.collected.clone()
    }
}

impl AudioSink for FakeSink {
    fn push(&mut self, pcm: Vec<i16>, _sample_rate: u32) -> Result<()> {
        self.collected.lock().unwrap().extend(pcm);
        Ok(())
    }

    fn stop(&mut self) {}
}

// ---------------------------------------------------------------------------
// FakeLlm — returns canned replies in order
// ---------------------------------------------------------------------------

pub struct FakeLlm {
    replies: Mutex<Vec<String>>,
}

impl FakeLlm {
    pub fn new(replies: Vec<String>) -> Self {
        Self {
            replies: Mutex::new(replies),
        }
    }
}

#[async_trait]
impl LlmCaller for FakeLlm {
    async fn chat(&self, _req: &ChatRequest) -> Result<ChatResponse> {
        let text = {
            let mut q = self.replies.lock().unwrap();
            if q.is_empty() {
                "(no more replies)".to_string()
            } else {
                q.remove(0)
            }
        };
        Ok(ChatResponse {
            id: Some("fake-id".to_string()),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: Role::Assistant,
                    content: Some(text),
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: "stop".to_string(),
            }],
            model: Some("fake-model".to_string()),
        })
    }
}

// ---------------------------------------------------------------------------
// FakeStt — returns scripted Final events per open_stream call
// ---------------------------------------------------------------------------

pub struct FakeStt {
    /// Each element is a list of Final texts for one stream.
    scripts: Mutex<Vec<Vec<String>>>,
}

impl FakeStt {
    pub fn new(scripts: Vec<Vec<String>>) -> Self {
        Self {
            scripts: Mutex::new(scripts),
        }
    }
}

#[async_trait]
impl SpeechToText for FakeStt {
    async fn open_stream(&self, _config: &SttConfig) -> Result<Box<dyn SttStream>> {
        let finals = {
            let mut q = self.scripts.lock().unwrap();
            if q.is_empty() {
                vec![]
            } else {
                q.remove(0)
            }
        };
        Ok(Box::new(ScriptedSttStream {
            finals: finals.into_iter().collect(),
        }))
    }
}

#[derive(Debug)]
struct ScriptedSttStream {
    finals: std::collections::VecDeque<String>,
}

#[async_trait]
impl SttStream for ScriptedSttStream {
    async fn send_frame(&mut self, _pcm: &[i16]) -> Result<()> {
        Ok(())
    }

    async fn end_of_utterance(&mut self) -> Result<()> {
        Ok(())
    }

    async fn next_event(&mut self) -> Option<TranscriptEvent> {
        let text = self.finals.pop_front()?;
        Some(TranscriptEvent::Final {
            text,
            words: vec![],
        })
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// FakeTts — yields a single chunk of silence, then None
// ---------------------------------------------------------------------------

pub struct FakeTts;

#[async_trait]
impl TextToSpeech for FakeTts {
    async fn open_stream(&self, _config: &TtsConfig) -> Result<Box<dyn TtsStream>> {
        Ok(Box::new(EmptyTtsStream { sent: false }))
    }

    async fn synthesize_batch(&self, texts: &[&str]) -> Result<Vec<Vec<i16>>> {
        Ok(texts.iter().map(|_| vec![0i16; 100]).collect())
    }
}

#[derive(Debug)]
struct EmptyTtsStream {
    sent: bool,
}

#[async_trait]
impl TtsStream for EmptyTtsStream {
    async fn push_text(&mut self, _chunk: &str) -> Result<()> {
        Ok(())
    }

    async fn end_of_input(&mut self) -> Result<()> {
        Ok(())
    }

    async fn next_chunk(&mut self) -> Option<Vec<i16>> {
        if self.sent {
            None
        } else {
            self.sent = true;
            Some(vec![0i16; 160]) // one frame of silence
        }
    }

    async fn abort(&mut self) -> Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// FakeMic helpers — keep a broadcast channel alive for MicGate
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub fn fake_mic_broadcast() -> (broadcast::Sender<Frame>, broadcast::Receiver<Frame>) {
    let (tx, rx) = broadcast::channel(16);
    (tx, rx)
}
