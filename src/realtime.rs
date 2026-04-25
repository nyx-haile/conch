use crate::stt::{TranscriptEvent, Word};
use anyhow::{anyhow, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};

pub const MAX_AUDIO_FRAME_ENCODED_BYTES: usize = 128 * 1024;
pub const MIN_AUDIO_SAMPLE_RATE: u32 = 8_000;
pub const MAX_AUDIO_SAMPLE_RATE: u32 = 48_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicMode {
    Hold,
    Tap,
    Button,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioEncoding {
    Pcm16,
    Opus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtsAudioCodec {
    Linear16,
    Opus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioFramePayload {
    pub encoding: AudioEncoding,
    pub sample_rate: u32,
    pub sequence: u64,
    /// Base64-encoded audio payload. The WebSocket gateway may transport audio
    /// in binary frames, but the contract keeps this field explicit for JSON
    /// validation tests and reconnect replay safety.
    pub chunk: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientEvent {
    #[serde(rename = "session.start")]
    SessionStart {
        topic: String,
        #[serde(default)]
        model_slug: Option<String>,
    },
    #[serde(rename = "consent.accept")]
    ConsentAccept { consent_version: String },
    #[serde(rename = "audio.frame")]
    AudioFrame(AudioFramePayload),
    #[serde(rename = "mic.toggle")]
    MicToggle { mode: MicMode },
    #[serde(rename = "assistant.interrupt")]
    AssistantInterrupt,
    #[serde(rename = "model.change")]
    ModelChange { model_slug: String },
    #[serde(rename = "session.export_requested")]
    SessionExportRequested,
    #[serde(rename = "session.close")]
    SessionClose,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerEvent {
    #[serde(rename = "status.changed")]
    StatusChanged {
        status: SessionStatus,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        banner: Option<String>,
    },
    #[serde(rename = "transcript.partial")]
    TranscriptPartial { text: String, stability: f32 },
    #[serde(rename = "transcript.final")]
    TranscriptFinal {
        turn_id: String,
        text: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        words: Vec<RealtimeWord>,
    },
    #[serde(rename = "assistant.delta")]
    AssistantDelta { text: String },
    #[serde(rename = "assistant.final")]
    AssistantFinal {
        turn_id: String,
        text: String,
        model_slug: String,
        usage: RealtimeUsage,
    },
    #[serde(rename = "tts.audio")]
    TtsAudio {
        codec: TtsAudioCodec,
        sample_rate: u32,
        chunk: String,
    },
    #[serde(rename = "waveform.level")]
    WaveformLevel { rms: f32 },
    #[serde(rename = "usage.updated")]
    UsageUpdated(RealtimeUsage),
    #[serde(rename = "error")]
    Error {
        code: String,
        retryable: bool,
        message: String,
    },
    #[serde(rename = "session.closed")]
    SessionClosed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealtimeWord {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

impl From<Word> for RealtimeWord {
    fn from(word: Word) -> Self {
        Self {
            text: word.text,
            start_ms: word.start_ms,
            end_ms: word.end_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RealtimeUsage {
    pub stt_seconds: u64,
    pub tts_chars: u64,
    pub llm_tokens: u64,
    pub credits_remaining: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolError {
    pub code: String,
    pub retryable: bool,
    pub message: String,
}

impl ProtocolError {
    pub fn new(code: impl Into<String>, retryable: bool, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            retryable,
            message: message.into(),
        }
    }

    pub fn into_event(self) -> ServerEvent {
        ServerEvent::Error {
            code: self.code,
            retryable: self.retryable,
            message: self.message,
        }
    }
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ProtocolError {}

#[derive(Debug, Clone)]
pub struct RealtimeSession {
    id: String,
    topic: Option<String>,
    selected_model_slug: Option<String>,
    consent_version: Option<String>,
    status: SessionStatus,
    next_audio_sequence: u64,
    next_turn_number: u64,
    usage: RealtimeUsage,
    closed: bool,
}

impl RealtimeSession {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            topic: None,
            selected_model_slug: None,
            consent_version: None,
            status: SessionStatus::Idle,
            next_audio_sequence: 0,
            next_turn_number: 1,
            usage: RealtimeUsage::default(),
            closed: false,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn status(&self) -> SessionStatus {
        self.status
    }

    pub fn usage(&self) -> &RealtimeUsage {
        &self.usage
    }

    pub fn selected_model_slug(&self) -> Option<&str> {
        self.selected_model_slug.as_deref()
    }

    pub fn consent_accepted(&self) -> bool {
        self.consent_version.is_some()
    }

    pub fn next_audio_sequence(&self) -> u64 {
        self.next_audio_sequence
    }

    pub fn handle_client_event(
        &mut self,
        event: ClientEvent,
    ) -> std::result::Result<Vec<ServerEvent>, ProtocolError> {
        if self.closed && !matches!(event, ClientEvent::SessionClose) {
            return Err(ProtocolError::new(
                "session_closed",
                false,
                "session is already closed",
            ));
        }

        match event {
            ClientEvent::SessionStart { topic, model_slug } => {
                let topic = topic.trim();
                if topic.is_empty() || topic.len() > 200 {
                    return Err(ProtocolError::new(
                        "invalid_topic",
                        false,
                        "session topic must be 1-200 characters",
                    ));
                }
                self.topic = Some(topic.to_string());
                self.selected_model_slug = model_slug.filter(|slug| !slug.trim().is_empty());
                self.set_status(SessionStatus::Idle, None)
            }
            ClientEvent::ConsentAccept { consent_version } => {
                let consent_version = consent_version.trim();
                if consent_version.is_empty() || consent_version.len() > 64 {
                    return Err(ProtocolError::new(
                        "invalid_consent",
                        false,
                        "consent version must be 1-64 characters",
                    ));
                }
                self.consent_version = Some(consent_version.to_string());
                self.set_status(
                    SessionStatus::Idle,
                    Some(format!("Recording consent accepted ({consent_version})")),
                )
            }
            ClientEvent::AudioFrame(frame) => self.handle_audio_frame(frame),
            ClientEvent::MicToggle { mode: _ } => {
                self.require_consent()?;
                self.set_status(SessionStatus::Listening, None)
            }
            ClientEvent::AssistantInterrupt => self.set_status(
                SessionStatus::Interrupted,
                Some("Assistant interrupted".into()),
            ),
            ClientEvent::ModelChange { model_slug } => {
                let model_slug = model_slug.trim();
                if model_slug.is_empty() || model_slug.len() > 160 {
                    return Err(ProtocolError::new(
                        "invalid_model",
                        false,
                        "model slug must be 1-160 characters",
                    ));
                }
                self.selected_model_slug = Some(model_slug.to_string());
                self.set_status(self.status, None)
            }
            ClientEvent::SessionExportRequested => Ok(Vec::new()),
            ClientEvent::SessionClose => {
                self.closed = true;
                self.status = SessionStatus::Closing;
                Ok(vec![
                    ServerEvent::StatusChanged {
                        status: SessionStatus::Closing,
                        banner: None,
                    },
                    ServerEvent::SessionClosed {
                        reason: "client_closed".into(),
                    },
                ])
            }
        }
    }

    pub fn apply_transcript_event(&mut self, event: TranscriptEvent) -> Vec<ServerEvent> {
        match event {
            TranscriptEvent::Partial { text, stability } => {
                vec![ServerEvent::TranscriptPartial { text, stability }]
            }
            TranscriptEvent::Final { text, words } => {
                let turn_id = self.next_turn_id();
                vec![ServerEvent::TranscriptFinal {
                    turn_id,
                    text,
                    words: words.into_iter().map(RealtimeWord::from).collect(),
                }]
            }
            TranscriptEvent::Error { message } => vec![ServerEvent::Error {
                code: "stt_error".into(),
                retryable: true,
                message,
            }],
        }
    }

    pub fn assistant_delta(&self, text: impl Into<String>) -> ServerEvent {
        ServerEvent::AssistantDelta { text: text.into() }
    }

    pub fn assistant_final(
        &mut self,
        text: impl Into<String>,
        model_slug: impl Into<String>,
        llm_tokens: u64,
    ) -> Vec<ServerEvent> {
        self.usage.llm_tokens = self.usage.llm_tokens.saturating_add(llm_tokens);
        let event = ServerEvent::AssistantFinal {
            turn_id: self.next_turn_id(),
            text: text.into(),
            model_slug: model_slug.into(),
            usage: self.usage.clone(),
        };
        vec![event, ServerEvent::UsageUpdated(self.usage.clone())]
    }

    pub fn tts_audio_event(&mut self, pcm: &[i16], sample_rate: u32) -> Result<Vec<ServerEvent>> {
        if sample_rate < MIN_AUDIO_SAMPLE_RATE || sample_rate > MAX_AUDIO_SAMPLE_RATE {
            return Err(anyhow!("unsupported TTS sample rate: {sample_rate}"));
        }
        let bytes: Vec<u8> = pcm.iter().flat_map(|sample| sample.to_le_bytes()).collect();
        let chunk = base64::engine::general_purpose::STANDARD.encode(bytes);
        Ok(vec![ServerEvent::TtsAudio {
            codec: TtsAudioCodec::Linear16,
            sample_rate,
            chunk,
        }])
    }

    pub fn record_tts_text(&mut self, text: &str) -> ServerEvent {
        self.usage.tts_chars = self
            .usage
            .tts_chars
            .saturating_add(text.chars().count() as u64);
        ServerEvent::UsageUpdated(self.usage.clone())
    }

    pub fn add_stt_audio_millis(&mut self, millis: u64) -> ServerEvent {
        self.usage.stt_seconds = self.usage.stt_seconds.saturating_add(millis.div_ceil(1000));
        ServerEvent::UsageUpdated(self.usage.clone())
    }

    fn handle_audio_frame(
        &mut self,
        frame: AudioFramePayload,
    ) -> std::result::Result<Vec<ServerEvent>, ProtocolError> {
        self.require_consent()?;
        if frame.sequence != self.next_audio_sequence {
            return Err(ProtocolError::new(
                "bad_sequence",
                true,
                format!(
                    "expected audio sequence {}, got {}",
                    self.next_audio_sequence, frame.sequence
                ),
            ));
        }
        if frame.sample_rate < MIN_AUDIO_SAMPLE_RATE || frame.sample_rate > MAX_AUDIO_SAMPLE_RATE {
            return Err(ProtocolError::new(
                "bad_sample_rate",
                false,
                "audio sample rate must be between 8000 and 48000 Hz",
            ));
        }
        if frame.chunk.is_empty() || frame.chunk.len() > MAX_AUDIO_FRAME_ENCODED_BYTES {
            return Err(ProtocolError::new(
                "bad_audio_frame",
                false,
                "audio frame is empty or too large",
            ));
        }

        let waveform_rms = if frame.encoding == AudioEncoding::Pcm16 {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(frame.chunk.as_bytes())
                .map_err(|_| {
                    ProtocolError::new(
                        "bad_audio_frame",
                        false,
                        "pcm16 audio frame must be valid base64",
                    )
                })?;
            if bytes.len() % 2 != 0 {
                return Err(ProtocolError::new(
                    "bad_audio_frame",
                    false,
                    "pcm16 audio frame must contain whole i16 samples",
                ));
            }
            Some(pcm16_rms(&bytes))
        } else {
            None
        };

        self.next_audio_sequence += 1;
        self.status = SessionStatus::Listening;
        let mut events = vec![ServerEvent::StatusChanged {
            status: SessionStatus::Listening,
            banner: None,
        }];
        if let Some(rms) = waveform_rms {
            events.push(ServerEvent::WaveformLevel { rms });
        }
        Ok(events)
    }

    fn require_consent(&self) -> std::result::Result<(), ProtocolError> {
        if self.consent_version.is_none() {
            return Err(ProtocolError::new(
                "consent_required",
                false,
                "recording consent is required before mic capture or provider streaming",
            ));
        }
        Ok(())
    }

    fn set_status(
        &mut self,
        status: SessionStatus,
        banner: Option<String>,
    ) -> std::result::Result<Vec<ServerEvent>, ProtocolError> {
        self.status = status;
        Ok(vec![ServerEvent::StatusChanged { status, banner }])
    }

    fn next_turn_id(&mut self) -> String {
        let id = format!("turn-{}", self.next_turn_number);
        self.next_turn_number += 1;
        id
    }
}

fn pcm16_rms(bytes: &[u8]) -> f32 {
    let mut sum_sq = 0.0f64;
    let mut count = 0usize;
    for chunk in bytes.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as f64 / i16::MAX as f64;
        sum_sq += sample * sample;
        count += 1;
    }
    if count == 0 {
        return 0.0;
    }
    (sum_sq / count as f64).sqrt().clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::pcm16_rms;

    #[test]
    fn rms_handles_empty_and_full_scale_pcm() {
        assert_eq!(pcm16_rms(&[]), 0.0);
        let full_scale = [i16::MAX.to_le_bytes(), i16::MAX.to_le_bytes()].concat();
        assert!((pcm16_rms(&full_scale) - 1.0).abs() < 0.001);
    }
}
