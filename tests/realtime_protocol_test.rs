use base64::Engine;
use conch::realtime::{
    AudioEncoding, AudioFramePayload, ClientEvent, RealtimeSession, ServerEvent, SessionStatus,
};
use conch::stt::{TranscriptEvent, Word};

fn pcm16_chunk(samples: &[i16]) -> String {
    let bytes: Vec<u8> = samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[test]
fn realtime_rejects_audio_until_recording_consent_is_accepted() {
    let mut session = RealtimeSession::new("sess-1");
    session
        .handle_client_event(ClientEvent::SessionStart {
            topic: "Launch interview".into(),
            model_slug: None,
        })
        .unwrap();

    let err = session
        .handle_client_event(ClientEvent::AudioFrame(AudioFramePayload {
            encoding: AudioEncoding::Pcm16,
            sample_rate: 16_000,
            sequence: 0,
            chunk: pcm16_chunk(&[0, 1, 2]),
        }))
        .unwrap_err();

    assert_eq!(err.code, "consent_required");
    assert_eq!(session.next_audio_sequence(), 0);
}

#[test]
fn realtime_accepts_monotonic_pcm_frames_and_emits_waveform() {
    let mut session = RealtimeSession::new("sess-1");
    session
        .handle_client_event(ClientEvent::SessionStart {
            topic: "Launch interview".into(),
            model_slug: Some("x-ai/grok-free".into()),
        })
        .unwrap();
    session
        .handle_client_event(ClientEvent::ConsentAccept {
            consent_version: "voice-v1".into(),
        })
        .unwrap();

    let events = session
        .handle_client_event(ClientEvent::AudioFrame(AudioFramePayload {
            encoding: AudioEncoding::Pcm16,
            sample_rate: 16_000,
            sequence: 0,
            chunk: pcm16_chunk(&[0, i16::MAX]),
        }))
        .unwrap();

    assert_eq!(session.status(), SessionStatus::Listening);
    assert_eq!(session.next_audio_sequence(), 1);
    assert!(events.iter().any(|event| matches!(
        event,
        ServerEvent::WaveformLevel { rms } if *rms > 0.70 && *rms <= 1.0
    )));

    let err = session
        .handle_client_event(ClientEvent::AudioFrame(AudioFramePayload {
            encoding: AudioEncoding::Pcm16,
            sample_rate: 16_000,
            sequence: 0,
            chunk: pcm16_chunk(&[0]),
        }))
        .unwrap_err();
    assert_eq!(err.code, "bad_sequence");
}

#[test]
fn realtime_rejects_malformed_pcm_before_sequence_advances() {
    let mut session = RealtimeSession::new("sess-1");
    session
        .handle_client_event(ClientEvent::ConsentAccept {
            consent_version: "voice-v1".into(),
        })
        .unwrap();

    let err = session
        .handle_client_event(ClientEvent::AudioFrame(AudioFramePayload {
            encoding: AudioEncoding::Pcm16,
            sample_rate: 16_000,
            sequence: 0,
            chunk: "not base64".into(),
        }))
        .unwrap_err();

    assert_eq!(err.code, "bad_audio_frame");
    assert_eq!(session.next_audio_sequence(), 0);
}

#[test]
fn realtime_maps_stt_events_to_ordered_transcript_events() {
    let mut session = RealtimeSession::new("sess-1");

    let partial = session.apply_transcript_event(TranscriptEvent::Partial {
        text: "hel".into(),
        stability: 0.7,
    });
    assert_eq!(
        partial,
        vec![ServerEvent::TranscriptPartial {
            text: "hel".into(),
            stability: 0.7
        }]
    );

    let final_events = session.apply_transcript_event(TranscriptEvent::Final {
        text: "hello".into(),
        words: vec![Word {
            text: "hello".into(),
            start_ms: 0,
            end_ms: 500,
        }],
    });

    assert!(matches!(
        &final_events[0],
        ServerEvent::TranscriptFinal { turn_id, text, words }
            if turn_id == "turn-1" && text == "hello" && words.len() == 1
    ));
}

#[test]
fn realtime_tracks_tts_chars_separately_from_stt_seconds() {
    let mut session = RealtimeSession::new("sess-1");
    let usage_event = session.record_tts_text("hello 🌊");

    assert!(matches!(
        usage_event,
        ServerEvent::UsageUpdated(ref usage)
            if usage.tts_chars == 7 && usage.stt_seconds == 0 && usage.llm_tokens == 0
    ));

    let audio_events = session.tts_audio_event(&[0, i16::MAX], 24_000).unwrap();
    assert!(matches!(
        &audio_events[0],
        ServerEvent::TtsAudio {
            sample_rate: 24_000,
            ..
        }
    ));
}
