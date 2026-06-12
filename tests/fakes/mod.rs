use anyhow::Result;
use async_trait::async_trait;
use conch::audio::input::Frame;
use conch::audio::output::AudioSink;
use conch::interview::orchestrator::LlmCaller;
use conch::llm::types::{ChatRequest, ChatResponse, Choice, Message, Role};
use conch::stt::{SpeechToText, SttConfig, SttStream, TranscriptEvent};
use conch::tts::{TextToSpeech, TtsConfig, TtsStream};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

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
// CountingLlm — wraps FakeLlm and counts chat() invocations
// ---------------------------------------------------------------------------

pub struct CountingLlm {
    inner: FakeLlm,
    pub count: Arc<std::sync::atomic::AtomicU32>,
}

impl CountingLlm {
    pub fn new(replies: Vec<String>) -> Self {
        Self {
            inner: FakeLlm::new(replies),
            count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }
}

#[async_trait]
impl LlmCaller for CountingLlm {
    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse> {
        self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.chat(req).await
    }
}

// ---------------------------------------------------------------------------
// SlowLlm — wraps FakeLlm with configurable per-call delay
// ---------------------------------------------------------------------------

pub struct SlowLlm {
    inner: FakeLlm,
    delay: std::time::Duration,
    call_count: std::sync::atomic::AtomicU32,
}

impl SlowLlm {
    /// The delay is applied starting from the second call (the opening call
    /// completes instantly so tests don't need to wait for it).
    pub fn new(replies: Vec<String>, delay: std::time::Duration) -> Self {
        Self {
            inner: FakeLlm::new(replies),
            delay,
            call_count: std::sync::atomic::AtomicU32::new(0),
        }
    }
}

#[async_trait]
impl LlmCaller for SlowLlm {
    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse> {
        let n = self
            .call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n > 0 {
            tokio::time::sleep(self.delay).await;
        }
        self.inner.chat(req).await
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

pub struct PersistentFinalStt {
    text: String,
}

impl PersistentFinalStt {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

#[async_trait]
impl SpeechToText for PersistentFinalStt {
    async fn open_stream(&self, _config: &SttConfig) -> Result<Box<dyn SttStream>> {
        Ok(Box::new(PersistentFinalStream {
            text: self.text.clone(),
            end_signalled: false,
            sent: false,
        }))
    }
}

#[derive(Debug)]
struct PersistentFinalStream {
    text: String,
    end_signalled: bool,
    sent: bool,
}

#[async_trait]
impl SttStream for PersistentFinalStream {
    async fn send_frame(&mut self, _pcm: &[i16]) -> Result<()> {
        Ok(())
    }

    async fn end_of_utterance(&mut self) -> Result<()> {
        self.end_signalled = true;
        Ok(())
    }

    async fn next_event(&mut self) -> Option<TranscriptEvent> {
        if !self.end_signalled || self.sent {
            std::future::pending().await
        } else {
            self.sent = true;
            Some(TranscriptEvent::Final {
                text: self.text.clone(),
                words: vec![],
            })
        }
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

pub struct EndlessPartialStt;

#[async_trait]
impl SpeechToText for EndlessPartialStt {
    async fn open_stream(&self, _config: &SttConfig) -> Result<Box<dyn SttStream>> {
        Ok(Box::new(EndlessPartialStream {
            end_signalled: false,
        }))
    }
}

#[derive(Debug)]
struct EndlessPartialStream {
    end_signalled: bool,
}

#[async_trait]
impl SttStream for EndlessPartialStream {
    async fn send_frame(&mut self, _pcm: &[i16]) -> Result<()> {
        Ok(())
    }

    async fn end_of_utterance(&mut self) -> Result<()> {
        self.end_signalled = true;
        Ok(())
    }

    async fn next_event(&mut self) -> Option<TranscriptEvent> {
        if !self.end_signalled {
            std::future::pending().await
        } else {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            Some(TranscriptEvent::Partial {
                text: "still draining".to_string(),
                stability: 0.1,
            })
        }
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
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

    fn sample_rate(&self) -> u32 {
        16_000
    }
}

// ---------------------------------------------------------------------------
// SlowTts — yields multiple chunks with a delay between each (for barge-in)
// ---------------------------------------------------------------------------

pub struct SlowTts {
    pub chunk_count: usize,
    pub chunk_delay: std::time::Duration,
}

#[async_trait]
impl TextToSpeech for SlowTts {
    async fn open_stream(&self, _config: &TtsConfig) -> Result<Box<dyn TtsStream>> {
        Ok(Box::new(SlowTtsStream {
            remaining: self.chunk_count,
            delay: self.chunk_delay,
            aborted: false,
        }))
    }

    async fn synthesize_batch(&self, texts: &[&str]) -> Result<Vec<Vec<i16>>> {
        Ok(texts.iter().map(|_| vec![0i16; 100]).collect())
    }
}

#[derive(Debug)]
struct SlowTtsStream {
    remaining: usize,
    delay: std::time::Duration,
    aborted: bool,
}

#[async_trait]
impl TtsStream for SlowTtsStream {
    async fn push_text(&mut self, _chunk: &str) -> Result<()> {
        Ok(())
    }

    async fn end_of_input(&mut self) -> Result<()> {
        Ok(())
    }

    async fn next_chunk(&mut self) -> Option<Vec<i16>> {
        if self.aborted || self.remaining == 0 {
            return None;
        }
        tokio::time::sleep(self.delay).await;
        self.remaining -= 1;
        Some(vec![0i16; 160])
    }

    async fn abort(&mut self) -> Result<()> {
        self.aborted = true;
        Ok(())
    }

    fn sample_rate(&self) -> u32 {
        16_000
    }
}

// ---------------------------------------------------------------------------
// PartialStt — emits Partial events with timing, then Final events after
// end_of_utterance. For speculative drafting tests.
// ---------------------------------------------------------------------------

/// A scripted STT event for PartialStt. Each event is emitted after a delay.
#[derive(Debug)]
pub struct ScriptedEvent {
    pub delay: std::time::Duration,
    pub event: TranscriptEvent,
}

pub struct PartialStt {
    scripts: Mutex<Vec<Vec<ScriptedEvent>>>,
}

impl PartialStt {
    pub fn new(scripts: Vec<Vec<ScriptedEvent>>) -> Self {
        Self {
            scripts: Mutex::new(scripts),
        }
    }
}

#[async_trait]
impl SpeechToText for PartialStt {
    async fn open_stream(&self, _config: &SttConfig) -> Result<Box<dyn SttStream>> {
        let events = {
            let mut q = self.scripts.lock().unwrap();
            if q.is_empty() {
                vec![]
            } else {
                q.remove(0)
            }
        };
        Ok(Box::new(TimedSttStream {
            events: events.into_iter().collect(),
            end_signalled: false,
        }))
    }
}

#[derive(Debug)]
struct TimedSttStream {
    events: std::collections::VecDeque<ScriptedEvent>,
    end_signalled: bool,
}

#[async_trait]
impl SttStream for TimedSttStream {
    async fn send_frame(&mut self, _pcm: &[i16]) -> Result<()> {
        Ok(())
    }

    async fn end_of_utterance(&mut self) -> Result<()> {
        self.end_signalled = true;
        Ok(())
    }

    async fn next_event(&mut self) -> Option<TranscriptEvent> {
        // If end_of_utterance hasn't been called yet, yield pending events
        // with their scripted delays. After end_of_utterance, yield remaining
        // events immediately (so drain_final_text can collect them).
        let se = self.events.pop_front()?;
        if !self.end_signalled {
            tokio::time::sleep(se.delay).await;
        }
        Some(se.event)
    }

    async fn close(&mut self) -> Result<()> {
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
