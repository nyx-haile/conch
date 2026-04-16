use crate::audio::output::AudioSink;
use crate::interview::fillers::{FillerCache, FillerCategory};
use crate::interview::history::{ConversationLog, Speaker, Turn};
use crate::interview::intent::{detect_end_command, end_session_tool};
use crate::interview::prompt::compose_system_prompt;
use crate::interview::speculative::{partial_is_stable_long_enough, should_commit_draft};
use crate::interview::tui::state::{AppState, Status, TurnView};
use crate::interview::tui::UserEvent;
use crate::llm::client::LlmClient;
use crate::llm::types::{ChatRequest, ChatResponse, Message, ToolCall};
use crate::stt::{SpeechToText, SttConfig, SttStream, TranscriptEvent};
use crate::tts::{TextToSpeech, TtsConfig};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
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
// SpeakOutcome — what happened when we tried to speak
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum SpeakOutcome {
    /// TTS finished naturally.
    Completed,
    /// User pressed Esc — playback was interrupted.
    Interrupted,
    /// User pressed Quit during playback.
    Quit,
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
    /// Pre-rendered filler audio for thinking pauses and interruptions.
    /// When `None`, no filler audio is played.
    pub fillers: Option<FillerCache>,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            model: "fake".to_string(),
            brief: String::new(),
            brand: None,
            max_tokens: 512,
            sample_rate: 16_000,
            fillers: None,
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
    log: Option<ConversationLog>,
    start_time: Instant,
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
            log: None,
            start_time: Instant::now(),
        }
    }

    /// Attach a conversation log for persisting turns to disk.
    pub fn with_log(mut self, log: ConversationLog) -> Self {
        self.log = Some(log);
        self
    }

    /// Run the full interview loop. Returns the reason the session ended.
    pub async fn run(mut self) -> Result<EndSignal> {
        let result = self.run_inner().await;
        if let Some(ref mut log) = self.log {
            if let Err(e) = log.finalize() {
                tracing::warn!(err = %e, "failed to finalize conversation log");
            }
        }
        result
    }

    async fn run_inner(&mut self) -> Result<EndSignal> {
        // Build the system prompt and seed the message history.
        let system = compose_system_prompt(&self.config.brief, self.config.brand.as_deref());
        self.messages.push(Message::system(system));

        // ---- Opening turn ----
        // Use a temporary seed message for the opening call only — don't persist
        // it in self.messages, as it would appear as a ghost user utterance.
        let opening = {
            let mut seed = self.messages.clone();
            seed.push(Message::user(
                "Begin the interview. Greet the user and ask your first question.",
            ));
            let req = ChatRequest {
                model: self.config.model.clone(),
                messages: seed,
                max_tokens: self.config.max_tokens,
                tools: vec![],
            };
            let resp = self.llm.chat(&req).await?;
            resp.choices
                .first()
                .and_then(|c| c.message.content.clone())
                .unwrap_or_default()
        };
        self.commit_assistant_turn(&opening).await;
        if self.speak_interruptible(&opening).await? == SpeakOutcome::Quit {
            self.set_status(Status::Closing).await;
            return Ok(EndSignal::UserQuit);
        }

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
                    // Nothing playing at idle — ignore.
                    continue;
                }
                UserEvent::MicToggle => {
                    // ---- Record phase (with speculative STT tracking) ----
                    self.set_status(Status::Listening).await;

                    // Open an STT stream.
                    let stt_config = SttConfig {
                        sample_rate: self.config.sample_rate,
                        language: None,
                        punctuate: true,
                    };
                    let mut stt_stream = self.stt.open_stream(&stt_config).await?;

                    // Speculative drafting state: track stable partials
                    // and optionally kick off a background LLM call.
                    let mut last_partial_text = String::new();
                    let mut partial_stable_since: Option<Instant> = None;
                    #[allow(clippy::type_complexity)]
                    let mut speculative_handle: Option<
                        tokio::task::JoinHandle<Result<(String, Vec<ToolCall>)>>,
                    > = None;
                    let mut speculative_partial = String::new();
                    let mut early_finals: Vec<String> = Vec::new();

                    // Poll user events AND STT partial events while recording.
                    // Once the STT stream yields None we stop polling it to
                    // avoid busy-looping (fakes return None immediately).
                    let mut stt_alive = true;
                    let stopped = loop {
                        if stt_alive {
                            tokio::select! {
                                biased; // prefer user events over STT partials
                                event = self.events.recv() => {
                                    match event {
                                        Some(UserEvent::MicToggle) => break false,
                                        Some(UserEvent::Quit) => break true,
                                        Some(UserEvent::Interrupt) => continue,
                                        None => break true,
                                    }
                                }
                                stt_event = stt_stream.next_event() => {
                                    match stt_event {
                                        Some(TranscriptEvent::Partial { text, stability }) => {
                                            let is_stable = stability >= 0.8;
                                            if text != last_partial_text {
                                                last_partial_text = text;
                                                partial_stable_since = if is_stable {
                                                    Some(Instant::now())
                                                } else {
                                                    None
                                                };
                                            } else if is_stable && partial_stable_since.is_none() {
                                                partial_stable_since = Some(Instant::now());
                                            }

                                            // Check whether we should launch a speculative call.
                                            if speculative_handle.is_none() && !last_partial_text.is_empty() {
                                                let stable_ms = partial_stable_since
                                                    .map(|t| t.elapsed().as_millis() as u64)
                                                    .unwrap_or(0);
                                                if partial_is_stable_long_enough(stable_ms, is_stable) {
                                                    let llm = Arc::clone(&self.llm);
                                                    let mut msgs = self.messages.clone();
                                                    msgs.push(Message::user(&last_partial_text));
                                                    let model = self.config.model.clone();
                                                    let max_tokens = self.config.max_tokens;
                                                    speculative_partial = last_partial_text.clone();

                                                    speculative_handle = Some(tokio::spawn(async move {
                                                        let req = ChatRequest {
                                                            model,
                                                            messages: msgs,
                                                            max_tokens,
                                                            tools: vec![end_session_tool()],
                                                        };
                                                        let resp = llm.chat(&req).await?;
                                                        let choice = resp.choices.first()
                                                            .ok_or_else(|| anyhow!("no choices"))?;
                                                        let text = choice.message.content.clone().unwrap_or_default();
                                                        let tool_calls = choice.message.tool_calls.clone().unwrap_or_default();
                                                        Ok((text, tool_calls))
                                                    }));
                                                }
                                            }
                                        }
                                        Some(TranscriptEvent::Final { text, .. }) => {
                                            // Rare during recording but possible.
                                            if !text.is_empty() {
                                                early_finals.push(text);
                                            }
                                        }
                                        Some(TranscriptEvent::Error { .. }) | None => {
                                            // STT stream ended or errored during recording.
                                            stt_alive = false;
                                        }
                                    }
                                }
                            }
                        } else {
                            // STT stream exhausted — only wait for user events.
                            match self.events.recv().await {
                                Some(UserEvent::MicToggle) => break false,
                                Some(UserEvent::Quit) => break true,
                                Some(UserEvent::Interrupt) => continue,
                                None => break true,
                            }
                        }
                    };

                    if stopped {
                        if let Some(h) = speculative_handle {
                            h.abort();
                        }
                        let _ = stt_stream.close().await;
                        self.set_status(Status::Closing).await;
                        return Ok(EndSignal::UserQuit);
                    }

                    // ---- Transcribe ----
                    self.set_status(Status::Thinking).await;
                    stt_stream.end_of_utterance().await?;

                    // Collect the final transcript (merge any finals captured
                    // during recording with those arriving after end_of_utterance).
                    let late_text = self.drain_final_text(&mut *stt_stream).await;
                    let _ = stt_stream.close().await;

                    let user_text = if early_finals.is_empty() {
                        late_text
                    } else if late_text.is_empty() {
                        early_finals.join(" ")
                    } else {
                        early_finals.push(late_text);
                        early_finals.join(" ")
                    };

                    if user_text.is_empty() {
                        // Nothing was said — go back to idle.
                        if let Some(h) = speculative_handle {
                            h.abort();
                        }
                        self.set_status(Status::Idle).await;
                        continue;
                    }

                    // Commit user turn.
                    self.commit_user_turn(&user_text).await;

                    // ---- Check voice end command ----
                    if detect_end_command(&user_text) {
                        if let Some(h) = speculative_handle {
                            h.abort();
                        }
                        // Ask LLM for a closing line using a temporary directive
                        // so we don't inject two consecutive user messages into history.
                        let closing = {
                            let mut closing_msgs = self.messages.clone();
                            closing_msgs.push(Message::user(
                                "The user wants to wrap up. Give a brief, warm closing.",
                            ));
                            let req = ChatRequest {
                                model: self.config.model.clone(),
                                messages: closing_msgs,
                                max_tokens: self.config.max_tokens,
                                tools: vec![],
                            };
                            let resp = self.llm.chat(&req).await?;
                            resp.choices
                                .first()
                                .and_then(|c| c.message.content.clone())
                                .unwrap_or_default()
                        };
                        self.commit_assistant_turn(&closing).await;
                        // Even the closing speak is interruptible.
                        if self.speak_interruptible(&closing).await? == SpeakOutcome::Quit {
                            self.set_status(Status::Closing).await;
                            return Ok(EndSignal::UserQuit);
                        }
                        self.set_status(Status::Closing).await;
                        return Ok(EndSignal::VoiceCommand);
                    }

                    // ---- LLM continuation (speculative or fresh, with filler race) ----
                    //
                    // If a speculative call was launched and its partial matches the
                    // final text closely enough, commit the speculative result directly.
                    // Otherwise, call the LLM fresh.
                    let (reply, tool_calls) = {
                        let speculative_result = if let Some(handle) = speculative_handle {
                            if should_commit_draft(&speculative_partial, &user_text) {
                                // Partial matched — try to use the speculative result.
                                match handle.await {
                                    Ok(Ok(result)) => Some(result),
                                    _ => None,
                                }
                            } else {
                                // Partial diverged — abort and call fresh.
                                handle.abort();
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(result) = speculative_result {
                            result
                        } else {
                            // Fresh LLM call with filler race.
                            let llm_future = self.call_llm_with_tools();
                            let filler_delay = tokio::time::sleep(Duration::from_millis(700));
                            tokio::pin!(llm_future);
                            tokio::pin!(filler_delay);

                            tokio::select! {
                                result = &mut llm_future => result?,
                                _ = &mut filler_delay => {
                                    self.set_status(Status::Filling).await;
                                    if let Some(ref fillers) = self.config.fillers {
                                        if let Some(pcm) = fillers.pick_random(FillerCategory::Thinking) {
                                            self.sink.lock().await.push(pcm, self.config.sample_rate)?;
                                        }
                                    }
                                    llm_future.await?
                                }
                            }
                        }
                    };

                    // Check if LLM invoked end_session tool.
                    if let Some(reason) = Self::extract_end_session(&tool_calls) {
                        // Speak whatever the LLM said as a closing.
                        let closing_text = if reply.is_empty() {
                            "Thanks for chatting! That was great.".to_string()
                        } else {
                            reply
                        };
                        self.commit_assistant_turn(&closing_text).await;
                        if self.speak_interruptible(&closing_text).await? == SpeakOutcome::Quit {
                            self.set_status(Status::Closing).await;
                            return Ok(EndSignal::UserQuit);
                        }
                        self.set_status(Status::Closing).await;
                        return Ok(EndSignal::LlmEndSession { reason });
                    }

                    self.commit_assistant_turn(&reply).await;
                    if self.speak_interruptible(&reply).await? == SpeakOutcome::Quit {
                        self.set_status(Status::Closing).await;
                        return Ok(EndSignal::UserQuit);
                    }
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
        if let Some(ref mut log) = self.log {
            let elapsed = self.start_time.elapsed().as_millis() as u64;
            if let Err(e) = log.append(Turn {
                speaker: Speaker::User,
                text: text.to_string(),
                timestamp_ms: elapsed,
                speculative_hit: false,
                interrupted: false,
                filler_played: None,
            }) {
                tracing::warn!(err = %e, "failed to log user turn");
            }
        }
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
        if let Some(ref mut log) = self.log {
            let elapsed = self.start_time.elapsed().as_millis() as u64;
            if let Err(e) = log.append(Turn {
                speaker: Speaker::Conch,
                text: text.to_string(),
                timestamp_ms: elapsed,
                speculative_hit: false,
                interrupted: false,
                filler_played: None,
            }) {
                tracing::warn!(err = %e, "failed to log assistant turn");
            }
        }
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
            match ev {
                TranscriptEvent::Final { text, .. } => {
                    if !text.is_empty() {
                        parts.push(text);
                    }
                }
                TranscriptEvent::Error { message } => {
                    tracing::warn!(err = %message, "STT error during drain_final_text");
                }
                _ => {}
            }
        }
        parts.join(" ")
    }

    /// Synthesize text via TTS and push to the audio sink.
    ///
    /// Playback is interruptible: while streaming chunks from TTS we also
    /// poll `self.events`. An `Interrupt` event aborts the TTS stream and
    /// stops the sink. A `Quit` event does the same and signals shutdown.
    async fn speak_interruptible(&mut self, text: &str) -> Result<SpeakOutcome> {
        self.set_status(Status::Speaking).await;
        let tts_config = TtsConfig { voice_id: None };
        let mut stream = self.tts.open_stream(&tts_config).await?;
        stream.push_text(text).await?;
        stream.end_of_input().await?;

        let outcome;
        loop {
            tokio::select! {
                chunk = stream.next_chunk() => {
                    match chunk {
                        Some(pcm) => {
                            self.sink.lock().await.push(pcm, self.config.sample_rate)?;
                        }
                        None => {
                            outcome = SpeakOutcome::Completed;
                            break;
                        }
                    }
                }
                event = self.events.recv() => {
                    match event {
                        Some(UserEvent::Interrupt) => {
                            let _ = stream.abort().await;
                            self.sink.lock().await.stop();
                            outcome = SpeakOutcome::Interrupted;
                            break;
                        }
                        Some(UserEvent::Quit) => {
                            let _ = stream.abort().await;
                            self.sink.lock().await.stop();
                            outcome = SpeakOutcome::Quit;
                            break;
                        }
                        Some(_) => {
                            // Ignore MicToggle during playback.
                        }
                        None => {
                            // Channel closed — treat as quit.
                            let _ = stream.abort().await;
                            self.sink.lock().await.stop();
                            outcome = SpeakOutcome::Quit;
                            break;
                        }
                    }
                }
            }
        }

        self.set_status(Status::Idle).await;
        Ok(outcome)
    }
}
