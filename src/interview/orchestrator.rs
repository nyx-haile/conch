use crate::audio::output::AudioSink;
use crate::interview::history::Speaker;
use crate::interview::intent::{detect_end_command, end_session_tool};
use crate::interview::prompt::compose_system_prompt;
use crate::interview::tui::state::{AppState, Status, TurnView};
use crate::interview::tui::UserEvent;
use crate::llm::client::LlmClient;
use crate::llm::types::{ChatRequest, ChatResponse, Message, ToolCall};
use crate::stt::{SpeechToText, SttConfig, SttStream, TranscriptEvent};
use crate::tts::{TextToSpeech, TtsConfig};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};

// ---------------------------------------------------------------------------
// LlmCaller trait — abstracts LlmClient for testability
// ---------------------------------------------------------------------------

#[async_trait]
pub trait LlmCaller: Send + Sync {
    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse>;
}

#[async_trait]
impl LlmCaller for LlmClient {
    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse> {
        LlmClient::chat(self, req).await
    }
}

// ---------------------------------------------------------------------------
// EndSignal — why the orchestrator stopped
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndSignal {
    /// The user said a wrap-up phrase detected by `detect_end_command`.
    VoiceCommand,
    /// The LLM called the `end_session` tool.
    LlmEndSession { reason: String },
    /// The user pressed Quit in the TUI.
    UserQuit,
}

// ---------------------------------------------------------------------------
// OrchestratorConfig — knobs the caller can tune
// ---------------------------------------------------------------------------

pub struct OrchestratorConfig {
    pub model: String,
    pub brief: String,
    pub brand: Option<String>,
    pub max_tokens: u32,
    pub sample_rate: u32,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            model: "fake".to_string(),
            brief: String::new(),
            brand: None,
            max_tokens: 512,
            sample_rate: 16_000,
        }
    }
}

// ---------------------------------------------------------------------------
// Orchestrator
// ---------------------------------------------------------------------------

pub struct Orchestrator {
    llm: Arc<dyn LlmCaller>,
    stt: Arc<dyn SpeechToText>,
    tts: Arc<dyn TextToSpeech>,
    sink: Arc<Mutex<Box<dyn AudioSink>>>,
    state: Arc<RwLock<AppState>>,
    events: mpsc::Receiver<UserEvent>,
    config: OrchestratorConfig,
    messages: Vec<Message>,
}

impl Orchestrator {
    pub fn new(
        llm: Arc<dyn LlmCaller>,
        stt: Arc<dyn SpeechToText>,
        tts: Arc<dyn TextToSpeech>,
        sink: Box<dyn AudioSink>,
        state: Arc<RwLock<AppState>>,
        events: mpsc::Receiver<UserEvent>,
        config: OrchestratorConfig,
    ) -> Self {
        Self {
            llm,
            stt,
            tts,
            sink: Arc::new(Mutex::new(sink)),
            state,
            events,
            config,
            messages: Vec::new(),
        }
    }

    /// Run the full interview loop. Returns the reason the session ended.
    pub async fn run(mut self) -> Result<EndSignal> {
        // Build the system prompt and seed the message history.
        let system = compose_system_prompt(&self.config.brief, self.config.brand.as_deref());
        self.messages.push(Message::system(system));
        self.messages.push(Message::user(
            "Begin the interview. Greet the user and ask your first question.",
        ));

        // ---- Opening turn ----
        let opening = self.call_llm().await?;
        self.commit_assistant_turn(&opening).await;
        self.speak(&opening).await?;

        // ---- Main loop ----
        loop {
            // Wait for the next user event.
            let event = self
                .events
                .recv()
                .await
                .ok_or_else(|| anyhow!("event channel closed"))?;

            match event {
                UserEvent::Quit => {
                    self.set_status(Status::Closing).await;
                    return Ok(EndSignal::UserQuit);
                }
                UserEvent::Interrupt => {
                    // Stop current playback; keep looping.
                    self.sink.lock().await.stop();
                    continue;
                }
                UserEvent::MicToggle => {
                    // ---- Record phase ----
                    self.set_status(Status::Listening).await;

                    // Open an STT stream.
                    let stt_config = SttConfig {
                        sample_rate: self.config.sample_rate,
                        language: None,
                        punctuate: true,
                    };
                    let mut stt_stream = self.stt.open_stream(&stt_config).await?;

                    // Wait for the next MicToggle (release) or Quit.
                    let stopped = loop {
                        match self.events.recv().await {
                            Some(UserEvent::MicToggle) => break false,
                            Some(UserEvent::Quit) => break true,
                            Some(UserEvent::Interrupt) => {
                                // Ignore interrupt during recording.
                                continue;
                            }
                            None => break true,
                        }
                    };

                    if stopped {
                        let _ = stt_stream.close().await;
                        self.set_status(Status::Closing).await;
                        return Ok(EndSignal::UserQuit);
                    }

                    // ---- Transcribe ----
                    self.set_status(Status::Thinking).await;
                    stt_stream.end_of_utterance().await?;

                    // Collect the final transcript.
                    let user_text = self.drain_final_text(&mut *stt_stream).await;
                    let _ = stt_stream.close().await;

                    if user_text.is_empty() {
                        // Nothing was said — go back to idle.
                        self.set_status(Status::Idle).await;
                        continue;
                    }

                    // Commit user turn.
                    self.commit_user_turn(&user_text).await;

                    // ---- Check voice end command ----
                    if detect_end_command(&user_text) {
                        // Ask LLM for a closing line.
                        self.messages.push(Message::user(
                            "The user wants to wrap up. Give a brief, warm closing.",
                        ));
                        let closing = self.call_llm().await?;
                        self.commit_assistant_turn(&closing).await;
                        self.speak(&closing).await?;
                        self.set_status(Status::Closing).await;
                        return Ok(EndSignal::VoiceCommand);
                    }

                    // ---- LLM continuation ----
                    let (reply, tool_calls) = self.call_llm_with_tools().await?;

                    // Check if LLM invoked end_session tool.
                    if let Some(reason) = Self::extract_end_session(&tool_calls) {
                        // Speak whatever the LLM said as a closing.
                        let closing_text = if reply.is_empty() {
                            "Thanks for chatting! That was great.".to_string()
                        } else {
                            reply
                        };
                        self.commit_assistant_turn(&closing_text).await;
                        self.speak(&closing_text).await?;
                        self.set_status(Status::Closing).await;
                        return Ok(EndSignal::LlmEndSession { reason });
                    }

                    self.commit_assistant_turn(&reply).await;
                    self.speak(&reply).await?;
                }
            }
        }
    }

    // ----- helpers -----

    async fn set_status(&self, s: Status) {
        self.state.write().await.set_status(s);
    }

    async fn commit_user_turn(&mut self, text: &str) {
        self.messages.push(Message::user(text));
        self.state.write().await.push_turn(TurnView {
            speaker: Speaker::User,
            text: text.to_string(),
        });
    }

    async fn commit_assistant_turn(&mut self, text: &str) {
        self.messages.push(Message {
            role: crate::llm::types::Role::Assistant,
            content: Some(text.to_string()),
            tool_calls: None,
            tool_call_id: None,
        });
        self.state.write().await.push_turn(TurnView {
            speaker: Speaker::Conch,
            text: text.to_string(),
        });
    }

    /// Call the LLM without tools (for opening / closing turns).
    async fn call_llm(&self) -> Result<String> {
        let req = ChatRequest {
            model: self.config.model.clone(),
            messages: self.messages.clone(),
            max_tokens: self.config.max_tokens,
            tools: vec![],
        };
        let resp = self.llm.chat(&req).await?;
        let text = resp
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();
        Ok(text)
    }

    /// Call the LLM with the `end_session` tool. Returns (text, tool_calls).
    async fn call_llm_with_tools(&self) -> Result<(String, Vec<ToolCall>)> {
        let req = ChatRequest {
            model: self.config.model.clone(),
            messages: self.messages.clone(),
            max_tokens: self.config.max_tokens,
            tools: vec![end_session_tool()],
        };
        let resp = self.llm.chat(&req).await?;
        let choice = resp.choices.first().ok_or_else(|| anyhow!("no choices"))?;
        let text = choice.message.content.clone().unwrap_or_default();
        let tool_calls = choice.message.tool_calls.clone().unwrap_or_default();
        Ok((text, tool_calls))
    }

    /// Check whether the tool calls include an `end_session` invocation.
    fn extract_end_session(tool_calls: &[ToolCall]) -> Option<String> {
        tool_calls.iter().find_map(|tc| {
            if tc.function.name == "end_session" {
                let reason = serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                    .ok()
                    .and_then(|v| v.get("reason")?.as_str().map(String::from))
                    .unwrap_or_else(|| "LLM ended session".to_string());
                Some(reason)
            } else {
                None
            }
        })
    }

    /// Drain all Final transcript events from the STT stream.
    async fn drain_final_text(&self, stream: &mut dyn SttStream) -> String {
        let mut parts = Vec::new();
        while let Some(ev) = stream.next_event().await {
            if let TranscriptEvent::Final { text, .. } = ev {
                if !text.is_empty() {
                    parts.push(text);
                }
            }
        }
        parts.join(" ")
    }

    /// Synthesize text via TTS and push to the audio sink.
    async fn speak(&self, text: &str) -> Result<()> {
        self.set_status(Status::Speaking).await;
        let tts_config = TtsConfig { voice_id: None };
        let mut stream = self.tts.open_stream(&tts_config).await?;
        stream.push_text(text).await?;
        stream.end_of_input().await?;
        while let Some(chunk) = stream.next_chunk().await {
            self.sink
                .lock()
                .await
                .push(chunk, self.config.sample_rate)?;
        }
        self.set_status(Status::Idle).await;
        Ok(())
    }
}
