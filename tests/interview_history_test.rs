use conch::interview::history::{ConversationLog, Speaker, Turn};
use tempfile::tempdir;

#[test]
fn conversation_log_serializes_turns_to_json() {
    let dir = tempdir().unwrap();
    let mut log = ConversationLog::new(
        dir.path().join("conversation.json"),
        dir.path().join("transcript.md"),
    )
    .unwrap();

    log.append(Turn {
        speaker: Speaker::Conch,
        text: "Hello, let's begin.".into(),
        timestamp_ms: 100,
        speculative_hit: false,
        interrupted: false,
        filler_played: None,
    })
    .unwrap();
    log.append(Turn {
        speaker: Speaker::User,
        text: "Hi there.".into(),
        timestamp_ms: 1_500,
        speculative_hit: true,
        interrupted: false,
        filler_played: Some("hmm…".into()),
    })
    .unwrap();
    log.finalize().unwrap();

    let json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("conversation.json")).unwrap(),
    )
    .unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["speaker"], "conch");
    assert_eq!(arr[1]["speculative_hit"], true);
    assert_eq!(arr[1]["filler_played"], "hmm…");

    let md = std::fs::read_to_string(dir.path().join("transcript.md")).unwrap();
    assert!(md.contains("**Conch:**"));
    assert!(md.contains("Hello, let's begin."));
    assert!(md.contains("**You:** Hi there."));
}

#[test]
fn transcript_flushes_after_each_append() {
    let dir = tempdir().unwrap();
    let mut log = ConversationLog::new(dir.path().join("c.json"), dir.path().join("t.md")).unwrap();
    log.append(Turn {
        speaker: Speaker::Conch,
        text: "a".into(),
        timestamp_ms: 0,
        speculative_hit: false,
        interrupted: false,
        filler_played: None,
    })
    .unwrap();
    // Should be readable without calling finalize (crash survival).
    let md = std::fs::read_to_string(dir.path().join("t.md")).unwrap();
    assert!(md.contains("a"));
}
