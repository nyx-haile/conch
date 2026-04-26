//! TUI-mirrored web UI component contracts.
//!
//! The browser implementation should render from these framework-agnostic
//! view models instead of inventing a second visual grammar. The source of
//! truth remains `src/interview/tui/widgets.rs`: top bar, 60/40 transcript and
//! sidebar split, Brief/waveform/status sidebar stack, and bottom keyboard
//! controls.

use super::contracts::{
    ui_token_contract, CreditsRemaining, UiTokenContract, WebSpeaker, WebStatus, CONCH_COLOR_TOKEN,
    CONCH_LABEL, STATUS_INFO_COLOR_TOKEN, STATUS_WARN_COLOR_TOKEN, USER_COLOR_TOKEN, USER_LABEL,
    WAVEFORM_BARS,
};
use crate::interview::tui::state::AppState;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const WEB_UI_CONTRACT_VERSION: &str = "2026-04-25";
pub const TRANSCRIPT_REGION_ID: &str = "transcript";
pub const SIDEBAR_REGION_ID: &str = "sidebar";
pub const BRIEF_REGION_ID: &str = "brief";
pub const WAVEFORM_REGION_ID: &str = "waveform";
pub const STATUS_REGION_ID: &str = "status";
pub const TOP_BAR_REGION_ID: &str = "top_bar";
pub const BOTTOM_CONTROLS_REGION_ID: &str = "bottom_controls";
pub const ASSISTANT_STREAM_CURSOR: &str = "▋";
pub const CONSENT_BANNER_COPY: &str =
    "Recording consent required before microphone capture or Deepgram streaming.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutBreakpoint {
    Desktop,
    Mobile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutRole {
    TopBar,
    Transcript,
    Sidebar,
    Brief,
    Waveform,
    Status,
    BottomControls,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidebarMobileBehavior {
    Inline,
    Accordion,
    Sticky,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LayoutRegion {
    pub id: &'static str,
    pub label: &'static str,
    pub role: LayoutRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop_percent: Option<u8>,
    pub mobile_order: u8,
    pub mobile_behavior: SidebarMobileBehavior,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InterviewLayoutContract {
    pub version: &'static str,
    pub tokens: UiTokenContract,
    pub desktop: Vec<LayoutRegion>,
    pub mobile: Vec<LayoutRegion>,
    pub sidebar_stack: Vec<&'static str>,
    pub bottom_controls: Vec<KeyboardControl>,
}

impl InterviewLayoutContract {
    pub fn desktop_region(&self, id: &str) -> Option<&LayoutRegion> {
        self.desktop.iter().find(|region| region.id == id)
    }

    pub fn mobile_region(&self, id: &str) -> Option<&LayoutRegion> {
        self.mobile.iter().find(|region| region.id == id)
    }
}

pub fn interview_layout_contract() -> InterviewLayoutContract {
    let tokens = ui_token_contract();
    InterviewLayoutContract {
        version: WEB_UI_CONTRACT_VERSION,
        desktop: vec![
            LayoutRegion {
                id: TOP_BAR_REGION_ID,
                label: "Conch: <topic/session title> · elapsed timer",
                role: LayoutRole::TopBar,
                desktop_percent: None,
                mobile_order: 0,
                mobile_behavior: SidebarMobileBehavior::Inline,
            },
            LayoutRegion {
                id: TRANSCRIPT_REGION_ID,
                label: "Transcript",
                role: LayoutRole::Transcript,
                desktop_percent: Some(tokens.desktop_transcript_percent),
                mobile_order: 2,
                mobile_behavior: SidebarMobileBehavior::Inline,
            },
            LayoutRegion {
                id: SIDEBAR_REGION_ID,
                label: "Brief / waveform / status",
                role: LayoutRole::Sidebar,
                desktop_percent: Some(tokens.desktop_sidebar_percent),
                mobile_order: 1,
                mobile_behavior: SidebarMobileBehavior::Inline,
            },
            LayoutRegion {
                id: BOTTOM_CONTROLS_REGION_ID,
                label: "Keyboard controls",
                role: LayoutRole::BottomControls,
                desktop_percent: None,
                mobile_order: 4,
                mobile_behavior: SidebarMobileBehavior::Sticky,
            },
        ],
        mobile: vec![
            LayoutRegion {
                id: TOP_BAR_REGION_ID,
                label: "Conch: <topic/session title> · elapsed timer",
                role: LayoutRole::TopBar,
                desktop_percent: None,
                mobile_order: 0,
                mobile_behavior: SidebarMobileBehavior::Inline,
            },
            LayoutRegion {
                id: STATUS_REGION_ID,
                label: "Status",
                role: LayoutRole::Status,
                desktop_percent: None,
                mobile_order: 1,
                mobile_behavior: SidebarMobileBehavior::Inline,
            },
            LayoutRegion {
                id: WAVEFORM_REGION_ID,
                label: "Waveform",
                role: LayoutRole::Waveform,
                desktop_percent: None,
                mobile_order: 1,
                mobile_behavior: SidebarMobileBehavior::Inline,
            },
            LayoutRegion {
                id: TRANSCRIPT_REGION_ID,
                label: "Transcript",
                role: LayoutRole::Transcript,
                desktop_percent: None,
                mobile_order: 2,
                mobile_behavior: SidebarMobileBehavior::Inline,
            },
            LayoutRegion {
                id: BRIEF_REGION_ID,
                label: "Brief",
                role: LayoutRole::Brief,
                desktop_percent: None,
                mobile_order: 3,
                mobile_behavior: SidebarMobileBehavior::Accordion,
            },
            LayoutRegion {
                id: BOTTOM_CONTROLS_REGION_ID,
                label: "Keyboard controls",
                role: LayoutRole::BottomControls,
                desktop_percent: None,
                mobile_order: 4,
                mobile_behavior: SidebarMobileBehavior::Sticky,
            },
        ],
        sidebar_stack: vec![BRIEF_REGION_ID, WAVEFORM_REGION_ID, STATUS_REGION_ID],
        bottom_controls: keyboard_controls(),
        tokens,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeyboardControl {
    pub key: &'static str,
    pub label: &'static str,
    pub action: &'static str,
}

pub fn keyboard_controls() -> Vec<KeyboardControl> {
    vec![
        KeyboardControl {
            key: "Hold Space",
            label: "talk",
            action: "mic.hold",
        },
        KeyboardControl {
            key: "Tap Space",
            label: "toggle mic",
            action: "mic.toggle",
        },
        KeyboardControl {
            key: "Esc",
            label: "interrupt",
            action: "assistant.interrupt",
        },
        KeyboardControl {
            key: "?",
            label: "help",
            action: "help.open",
        },
        KeyboardControl {
            key: "E",
            label: "export",
            action: "session.export_requested",
        },
    ]
}

pub fn bottom_control_text() -> String {
    let mut parts = keyboard_controls()
        .into_iter()
        .take(4)
        .map(|control| format!("[{}] {}", control.key, control.label))
        .collect::<Vec<_>>();
    parts.push("[E] export".to_string());
    format!(" {} ", parts.join("  "))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptLineKind {
    Committed,
    Draft,
    Streaming,
    Cursor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpeakerStyle {
    pub speaker: WebSpeaker,
    pub label: &'static str,
    pub color_token: &'static str,
}

pub fn speaker_style(speaker: WebSpeaker) -> SpeakerStyle {
    match speaker {
        WebSpeaker::Conch => SpeakerStyle {
            speaker,
            label: CONCH_LABEL,
            color_token: CONCH_COLOR_TOKEN,
        },
        WebSpeaker::User => SpeakerStyle {
            speaker,
            label: USER_LABEL,
            color_token: USER_COLOR_TOKEN,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TranscriptLine {
    pub kind: TranscriptLineKind,
    pub speaker: WebSpeaker,
    pub label: &'static str,
    pub color_token: &'static str,
    pub text: String,
}

impl TranscriptLine {
    fn new(kind: TranscriptLineKind, speaker: WebSpeaker, text: impl Into<String>) -> Self {
        let style = speaker_style(speaker);
        Self {
            kind,
            speaker,
            label: style.label,
            color_token: style.color_token,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusView {
    pub status: WebStatus,
    pub glyph: &'static str,
    pub text: String,
    pub color_token: &'static str,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
}

pub fn status_view(status: WebStatus, banner: Option<&str>) -> StatusView {
    if let Some(banner) = banner {
        return StatusView {
            status,
            glyph: "!",
            text: banner.to_string(),
            color_token: STATUS_WARN_COLOR_TOKEN,
            banner: Some(banner.to_string()),
        };
    }

    let (glyph, text, color_token) = match status {
        WebStatus::Idle => ("✶", "Idle — hold Space to talk", "status-gray"),
        WebStatus::Listening => ("✻", "Listening…", USER_COLOR_TOKEN),
        WebStatus::Thinking | WebStatus::Filling => ("✶", "Thinking…", STATUS_WARN_COLOR_TOKEN),
        WebStatus::Speaking => ("✣", "Speaking…", CONCH_COLOR_TOKEN),
        WebStatus::Filler => ("✶", "…", STATUS_WARN_COLOR_TOKEN),
        WebStatus::Closing => ("✣", "Wrapping up…", CONCH_COLOR_TOKEN),
        WebStatus::Interrupted => ("✕", "Interrupted", STATUS_WARN_COLOR_TOKEN),
        WebStatus::Reconnecting => ("↻", "Reconnecting…", STATUS_INFO_COLOR_TOKEN),
        WebStatus::Error => ("!", "Error — retry or reconnect", STATUS_WARN_COLOR_TOKEN),
    };

    StatusView {
        status,
        glyph,
        text: text.to_string(),
        color_token,
        banner: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TopBarView {
    pub title: String,
    pub elapsed: String,
    pub usage_chip: String,
    pub model_chip: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConsentBannerView {
    pub required_before_mic_capture: bool,
    pub accepted: bool,
    pub copy: &'static str,
    pub action_event: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InterviewViewModel {
    pub layout: LayoutBreakpoint,
    pub top_bar: TopBarView,
    pub transcript: Vec<TranscriptLine>,
    pub brief: String,
    pub waveform: String,
    pub status: StatusView,
    pub controls: Vec<KeyboardControl>,
    pub consent_banner: ConsentBannerView,
    pub reconnect_banner: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebInterviewUiInput {
    pub model_display_name: String,
    pub credits_remaining: CreditsRemaining,
    pub recording_consent_accepted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reconnect_banner: Option<String>,
}

impl WebInterviewUiInput {
    pub fn trial_default(model_display_name: impl Into<String>) -> Self {
        Self {
            model_display_name: model_display_name.into(),
            credits_remaining: CreditsRemaining::default(),
            recording_consent_accepted: false,
            reconnect_banner: None,
        }
    }
}

impl InterviewViewModel {
    pub fn from_tui_state(
        state: &AppState,
        elapsed: Duration,
        layout: LayoutBreakpoint,
        input: WebInterviewUiInput,
    ) -> Self {
        let transcript = transcript_from_tui_state(state);
        let waveform = render_waveform(state.waveform(), 12);
        let status = status_view(WebStatus::from(state.status()), state.banner());

        Self {
            layout,
            top_bar: TopBarView {
                title: format!("Conch: {}", state.title()),
                elapsed: state.format_elapsed(elapsed),
                usage_chip: usage_chip(input.credits_remaining),
                model_chip: input.model_display_name,
            },
            transcript,
            brief: state.brief().to_string(),
            waveform,
            status,
            controls: keyboard_controls(),
            consent_banner: ConsentBannerView {
                required_before_mic_capture: true,
                accepted: input.recording_consent_accepted,
                copy: CONSENT_BANNER_COPY,
                action_event: "consent.accept",
            },
            reconnect_banner: input.reconnect_banner,
        }
    }
}

pub fn transcript_from_tui_state(state: &AppState) -> Vec<TranscriptLine> {
    let mut lines = Vec::new();

    for turn in state.history() {
        lines.push(TranscriptLine::new(
            TranscriptLineKind::Committed,
            WebSpeaker::from(turn.speaker),
            turn.text.clone(),
        ));
    }

    if !state.current_user_draft().is_empty() {
        lines.push(TranscriptLine::new(
            TranscriptLineKind::Draft,
            WebSpeaker::User,
            state.current_user_draft().to_string(),
        ));
    }

    if !state.assistant_stream().is_empty() {
        lines.push(TranscriptLine::new(
            TranscriptLineKind::Streaming,
            WebSpeaker::Conch,
            state.assistant_stream().to_string(),
        ));
        lines.push(TranscriptLine::new(
            TranscriptLineKind::Cursor,
            WebSpeaker::Conch,
            ASSISTANT_STREAM_CURSOR,
        ));
    }

    lines
}

pub fn render_waveform(samples: &[f32], cols: usize) -> String {
    if samples.is_empty() || cols == 0 {
        return String::new();
    }

    let step = (samples.len() as f32 / cols as f32).max(1.0);
    let mut rendered = String::with_capacity(cols);
    let mut index = 0.0;

    while (index as usize) < samples.len() && rendered.chars().count() < cols {
        let value = samples[index as usize].clamp(0.0, 1.0);
        let bar_index = (value * 7.0).round() as usize;
        rendered.push_str(WAVEFORM_BARS[bar_index.min(7)]);
        index += step;
    }

    rendered
}

pub fn usage_chip(credits: CreditsRemaining) -> String {
    format!(
        "{}m STT · {}k TTS · ${:.2} LLM",
        credits.stt_seconds / 60,
        credits.tts_chars / 1_000,
        credits.llm_budget_cents as f32 / 100.0
    )
}
