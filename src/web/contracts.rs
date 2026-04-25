//! Shared web launch protocol and TUI-parity contracts.
//!
//! The web MVP uses JSON messages over an authenticated WebSocket for control
//! and low-volume audio chunks. Audio chunks in these contracts are base64
//! strings so browser/server tests can validate schemas without exposing any
//! provider credentials or relying on provider SDKs.

use crate::interview::history::Speaker;
use crate::interview::tui::state::Status;
use serde::{Deserialize, Serialize};

pub const WEB_CONTRACT_VERSION: &str = "2026-04-25";
pub const CONCH_LABEL: &str = "✣ Conch";
pub const USER_LABEL: &str = "● You";
pub const CONCH_COLOR_TOKEN: &str = "conch-magenta";
pub const USER_COLOR_TOKEN: &str = "user-green";
pub const STATUS_INFO_COLOR_TOKEN: &str = "status-cyan";
pub const STATUS_WARN_COLOR_TOKEN: &str = "status-yellow";
pub const DESKTOP_TRANSCRIPT_PERCENT: u8 = 60;
pub const DESKTOP_SIDEBAR_PERCENT: u8 = 40;
pub const WAVEFORM_BARS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebStatus {
    Idle,
    Listening,
    Thinking,
    Filling,
    Speaking,
    Filler,
    Closing,
    Interrupted,
    Reconnecting,
    Error,
}

impl From<Status> for WebStatus {
    fn from(status: Status) -> Self {
        match status {
            Status::Idle => Self::Idle,
            Status::Listening => Self::Listening,
            Status::Thinking => Self::Thinking,
            Status::Filling => Self::Filling,
            Status::Speaking => Self::Speaking,
            Status::Filler => Self::Filler,
            Status::Closing => Self::Closing,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSpeaker {
    Conch,
    User,
}

impl From<Speaker> for WebSpeaker {
    fn from(speaker: Speaker) -> Self {
        match speaker {
            Speaker::Conch => Self::Conch,
            Speaker::User => Self::User,
        }
    }
}

impl WebSpeaker {
    pub fn label(self) -> &'static str {
        match self {
            Self::Conch => CONCH_LABEL,
            Self::User => USER_LABEL,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiTokenContract {
    pub conch_label: &'static str,
    pub user_label: &'static str,
    pub conch_color: &'static str,
    pub user_color: &'static str,
    pub status_info_color: &'static str,
    pub status_warn_color: &'static str,
    pub waveform_bars: [&'static str; 8],
    pub desktop_transcript_percent: u8,
    pub desktop_sidebar_percent: u8,
}

pub fn ui_token_contract() -> UiTokenContract {
    UiTokenContract {
        conch_label: CONCH_LABEL,
        user_label: USER_LABEL,
        conch_color: CONCH_COLOR_TOKEN,
        user_color: USER_COLOR_TOKEN,
        status_info_color: STATUS_INFO_COLOR_TOKEN,
        status_warn_color: STATUS_WARN_COLOR_TOKEN,
        waveform_bars: WAVEFORM_BARS,
        desktop_transcript_percent: DESKTOP_TRANSCRIPT_PERCENT,
        desktop_sidebar_percent: DESKTOP_SIDEBAR_PERCENT,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientEvent {
    #[serde(rename = "session.start")]
    SessionStart(SessionStart),
    #[serde(rename = "consent.accept")]
    ConsentAccept(ConsentAccept),
    #[serde(rename = "audio.frame")]
    AudioFrame(AudioFrame),
    #[serde(rename = "mic.toggle")]
    MicToggle(MicToggle),
    #[serde(rename = "assistant.interrupt")]
    AssistantInterrupt,
    #[serde(rename = "model.change")]
    ModelChange(ModelChange),
    #[serde(rename = "session.export_requested")]
    SessionExportRequested,
    #[serde(rename = "session.close")]
    SessionClose,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerEvent {
    #[serde(rename = "status.changed")]
    StatusChanged(StatusChanged),
    #[serde(rename = "transcript.partial")]
    TranscriptPartial(TranscriptPartial),
    #[serde(rename = "transcript.final")]
    TranscriptFinal(TranscriptFinal),
    #[serde(rename = "assistant.delta")]
    AssistantDelta(AssistantDelta),
    #[serde(rename = "assistant.final")]
    AssistantFinal(AssistantFinal),
    #[serde(rename = "tts.audio")]
    TtsAudio(TtsAudio),
    #[serde(rename = "waveform.level")]
    WaveformLevel(WaveformLevel),
    #[serde(rename = "usage.updated")]
    UsageUpdated(UsageUpdated),
    #[serde(rename = "error")]
    Error(ErrorEvent),
    #[serde(rename = "session.closed")]
    SessionClosed(SessionClosed),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStart {
    pub topic: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_slug: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentAccept {
    pub consent_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioCodec {
    Pcm16,
    Opus,
    WebmOpus,
    Mp3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioFrame {
    pub codec: AudioCodec,
    pub sample_rate: u32,
    pub sequence: u64,
    /// Base64-encoded audio bytes for JSON transport tests. Production can
    /// route larger audio via binary WebSocket frames while preserving the
    /// same metadata contract.
    pub chunk: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicMode {
    Hold,
    Tap,
    Button,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicToggle {
    pub mode: MicMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelChange {
    pub model_slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusChanged {
    pub status: WebStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptPartial {
    pub text: String,
    pub stability: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptFinal {
    pub turn_id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<TranscriptWord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptWord {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantDelta {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantFinal {
    pub turn_id: String,
    pub text: String,
    pub model_slug: String,
    pub usage: LlmTurnUsage,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmTurnUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_cents: u64,
}

impl LlmTurnUsage {
    pub fn total_tokens(self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TtsAudio {
    pub codec: AudioCodec,
    pub sample_rate: u32,
    /// Base64-encoded audio chunk.
    pub chunk: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WaveformLevel {
    pub rms: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditsRemaining {
    pub stt_seconds: u64,
    pub tts_chars: u64,
    pub llm_budget_cents: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageUpdated {
    pub stt_seconds: u64,
    pub tts_chars: u64,
    pub llm_tokens: u64,
    pub credits_remaining: CreditsRemaining,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Unauthorized,
    ConsentRequired,
    InsufficientCredits,
    ModelNotAllowed,
    ProviderUnavailable,
    RateLimited,
    InvalidEvent,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub code: ErrorCode,
    pub retryable: bool,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseReason {
    Complete,
    UserClosed,
    IdleTimeout,
    ReplacedByReconnect,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionClosed {
    pub reason: CloseReason,
}
