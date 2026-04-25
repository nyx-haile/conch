use conch::interview::history::Speaker;
use conch::interview::tui::state::{AppState, Status, TurnView};
use conch::web::*;
use std::time::Duration;

#[test]
fn desktop_layout_preserves_tui_regions_and_ratio() {
    let layout = interview_layout_contract();

    assert_eq!(layout.version, WEB_UI_CONTRACT_VERSION);
    assert_eq!(
        layout
            .desktop_region(TRANSCRIPT_REGION_ID)
            .and_then(|region| region.desktop_percent),
        Some(60)
    );
    assert_eq!(
        layout
            .desktop_region(SIDEBAR_REGION_ID)
            .and_then(|region| region.desktop_percent),
        Some(40)
    );
    assert_eq!(
        layout.sidebar_stack,
        vec![BRIEF_REGION_ID, WAVEFORM_REGION_ID, STATUS_REGION_ID]
    );
    assert_eq!(layout.tokens.conch_label, "✣ Conch");
    assert_eq!(layout.tokens.user_label, "● You");
    assert!(bottom_control_text().contains("[Hold Space] talk"));
    assert!(bottom_control_text().contains("[Tap Space] toggle mic"));
    assert!(bottom_control_text().contains("[Esc] interrupt"));
}

#[test]
fn mobile_layout_keeps_status_waveform_first_and_controls_sticky() {
    let layout = interview_layout_contract();

    assert_eq!(
        layout.mobile_region(STATUS_REGION_ID).unwrap().mobile_order,
        1
    );
    assert_eq!(
        layout
            .mobile_region(WAVEFORM_REGION_ID)
            .unwrap()
            .mobile_order,
        1
    );
    assert_eq!(
        layout
            .mobile_region(BRIEF_REGION_ID)
            .unwrap()
            .mobile_behavior,
        SidebarMobileBehavior::Accordion
    );
    assert_eq!(
        layout
            .mobile_region(BOTTOM_CONTROLS_REGION_ID)
            .unwrap()
            .mobile_behavior,
        SidebarMobileBehavior::Sticky
    );
}

#[test]
fn view_model_from_tui_state_preserves_speaker_labels_stream_cursor_and_consent_gate() {
    let mut state = AppState::new(
        "Repository interview".to_string(),
        "Ask about the launch plan and note risks.".to_string(),
    );
    state.push_turn(TurnView {
        speaker: Speaker::Conch,
        text: "Opening question...".to_string(),
    });
    state.update_current_user_draft("I would ship the web MVP first");
    state.update_assistant_stream("Great, let's probe the risk");
    state.set_status(Status::Listening);
    state.set_waveform(vec![0.0, 0.14, 0.28, 0.42, 0.56, 0.70, 0.84, 1.0]);

    let model = InterviewViewModel::from_tui_state(
        &state,
        Duration::from_secs(17 * 60 + 42),
        LayoutBreakpoint::Desktop,
        WebInterviewUiInput {
            model_display_name: "Grok Fast · Free".to_string(),
            credits_remaining: CreditsRemaining {
                stt_seconds: 7_200,
                tts_chars: 60_000,
                llm_budget_cents: 500,
            },
            recording_consent_accepted: false,
            reconnect_banner: None,
        },
    );

    assert_eq!(model.top_bar.title, "Conch: Repository interview");
    assert_eq!(model.top_bar.elapsed, "00:17:42");
    assert_eq!(model.top_bar.model_chip, "Grok Fast · Free");
    assert_eq!(model.brief, "Ask about the launch plan and note risks.");
    assert_eq!(model.status.text, "Listening…");
    assert!(model.waveform.contains('▁'));
    assert!(model.waveform.contains('█'));
    assert!(model.consent_banner.required_before_mic_capture);
    assert!(!model.consent_banner.accepted);
    assert_eq!(model.consent_banner.action_event, "consent.accept");

    let aggregate = model
        .transcript
        .iter()
        .map(|line| format!("{} {}", line.label, line.text))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(aggregate.contains("✣ Conch Opening question"));
    assert!(aggregate.contains("● You I would ship"));
    assert!(aggregate.contains("✣ Conch Great"));
    assert!(aggregate.contains("✣ Conch ▋"));
}

#[test]
fn status_views_match_tui_copy_and_cover_web_only_states() {
    assert_eq!(
        status_view(WebStatus::Idle, None).text,
        "Idle — hold Space to talk"
    );
    assert_eq!(status_view(WebStatus::Thinking, None).text, "Thinking…");
    assert_eq!(status_view(WebStatus::Filling, None).text, "Thinking…");
    assert_eq!(status_view(WebStatus::Speaking, None).glyph, "✣");
    assert_eq!(status_view(WebStatus::Closing, None).text, "Wrapping up…");

    let reconnecting = status_view(WebStatus::Reconnecting, None);
    assert_eq!(reconnecting.glyph, "↻");
    assert_eq!(reconnecting.color_token, STATUS_INFO_COLOR_TOKEN);

    let banner = status_view(WebStatus::Error, Some("Network degraded"));
    assert_eq!(banner.text, "Network degraded");
    assert_eq!(banner.color_token, STATUS_WARN_COLOR_TOKEN);
}

#[test]
fn view_model_serializes_stable_component_contract_for_frontend() {
    let state = AppState::new("Demo".to_string(), "Brief".to_string());
    let model = InterviewViewModel::from_tui_state(
        &state,
        Duration::from_secs(0),
        LayoutBreakpoint::Mobile,
        WebInterviewUiInput::trial_default("Grok Fast · Free"),
    );
    let value = serde_json::to_value(model).unwrap();

    assert_eq!(value["layout"], "mobile");
    assert_eq!(value["top_bar"]["title"], "Conch: Demo");
    assert_eq!(
        value["consent_banner"]["copy"],
        "Recording consent required before microphone capture or Deepgram streaming."
    );
    assert_eq!(
        value["controls"][0]["action"], "mic.hold",
        "first control should remain Hold Space talk"
    );
}
