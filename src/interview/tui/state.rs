use crate::interview::history::{Speaker, Turn};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Listening,
    Thinking,
    Filling,
    Speaking,
    Filler,
    Closing,
}

#[derive(Debug, Clone)]
pub struct TurnView {
    pub speaker: Speaker,
    pub text: String,
}

#[derive(Debug)]
pub struct AppState {
    title: String,
    brief: String,
    status: Status,
    history: Vec<TurnView>,
    current_user_draft: String,
    assistant_stream: String,
    waveform: Vec<f32>,
    banner: Option<String>,
}

impl AppState {
    pub fn new(title: String, brief: String) -> Self {
        Self {
            title,
            brief,
            status: Status::Idle,
            history: Vec::new(),
            current_user_draft: String::new(),
            assistant_stream: String::new(),
            waveform: Vec::new(),
            banner: None,
        }
    }

    pub fn title(&self) -> &str { &self.title }
    pub fn brief(&self) -> &str { &self.brief }
    pub fn status(&self) -> Status { self.status }
    pub fn history(&self) -> &[TurnView] { &self.history }
    pub fn current_user_draft(&self) -> &str { &self.current_user_draft }
    pub fn assistant_stream(&self) -> &str { &self.assistant_stream }
    pub fn waveform(&self) -> &[f32] { &self.waveform }
    pub fn banner(&self) -> Option<&str> { self.banner.as_deref() }

    pub fn set_status(&mut self, s: Status) { self.status = s; }
    pub fn set_banner(&mut self, b: Option<String>) { self.banner = b; }
    pub fn set_waveform(&mut self, samples: Vec<f32>) { self.waveform = samples; }

    pub fn update_current_user_draft(&mut self, text: impl Into<String>) {
        self.current_user_draft = text.into();
    }

    pub fn push_turn(&mut self, t: TurnView) {
        if matches!(t.speaker, Speaker::User) {
            self.current_user_draft.clear();
        }
        self.history.push(t);
    }

    pub fn update_assistant_stream(&mut self, text: impl Into<String>) {
        self.assistant_stream = text.into();
    }

    pub fn commit_assistant(&mut self) {
        if !self.assistant_stream.is_empty() {
            self.history.push(TurnView {
                speaker: Speaker::Conch,
                text: std::mem::take(&mut self.assistant_stream),
            });
        }
    }

    pub fn record_turn(&mut self, turn: &Turn) {
        if matches!(turn.speaker, Speaker::User) {
            self.current_user_draft.clear();
        }
        self.history.push(TurnView {
            speaker: turn.speaker,
            text: turn.text.clone(),
        });
    }

    pub fn format_elapsed(&self, d: Duration) -> String {
        let secs = d.as_secs();
        format!("{:02}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60)
    }
}
