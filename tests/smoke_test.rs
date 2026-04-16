use assert_cmd::Command;

/// End-to-end smoke test: runs the `test` subcommand in headless mode with
/// scripted LLM and STT backends. Verifies that session artifacts
/// (brief.md, conversation.json, transcript.md) are written to disk.
#[test]
fn headless_smoke_test_produces_session_artifacts() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path();

    // Pre-create the .conch directory so Config::load() works.
    std::fs::create_dir_all(home.join(".conch/sessions")).unwrap();

    // Scripted LLM responses (pipe-delimited):
    //   1. prep stage (run_agent calls chat, finish_reason "stop" -> returns text)
    //   2. orchestrator opening turn
    //   3. orchestrator closing turn (after "that's a wrap" voice command)
    let scripted_llm = "A brief about the project|Welcome! Tell me about yourself.|Great chat, thanks!";

    // Scripted STT finals: the user says "that's a wrap" which triggers
    // detect_end_command -> VoiceCommand end.
    let scripted_stt = "that's a wrap";

    Command::cargo_bin("conch")
        .unwrap()
        .arg("test")
        .env("HOME", home)
        .env("CONCH_HEADLESS", "1")
        .env("CONCH_PROVIDER", "anthropic")
        .env("ANTHROPIC_API_KEY", "sk-test-fake")
        .env("CONCH_TEST_SCRIPTED_LLM", scripted_llm)
        .env("CONCH_TEST_SCRIPTED_STT_FINALS", scripted_stt)
        .env("CONCH_STT", "local")
        .env("CONCH_TTS", "text")
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();

    // Find the session directory that was created.
    let sessions_dir = home.join(".conch/sessions");
    let entries: Vec<_> = std::fs::read_dir(&sessions_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    assert!(
        !entries.is_empty(),
        "expected at least one session directory in {}",
        sessions_dir.display()
    );

    let session_dir = &entries[0].path();

    // Verify artifacts exist.
    let brief_path = session_dir.join("brief.md");
    assert!(
        brief_path.exists(),
        "brief.md should exist at {}",
        brief_path.display()
    );
    let brief = std::fs::read_to_string(&brief_path).unwrap();
    assert!(
        brief.contains("A brief about the project"),
        "brief.md should contain scripted prep output, got: {brief}"
    );

    let convo_path = session_dir.join("conversation.json");
    assert!(
        convo_path.exists(),
        "conversation.json should exist at {}",
        convo_path.display()
    );
    let convo = std::fs::read_to_string(&convo_path).unwrap();
    // Parse as JSON to verify it's valid.
    let turns: Vec<serde_json::Value> = serde_json::from_str(&convo)
        .unwrap_or_else(|e| panic!("conversation.json should be valid JSON: {e}\nContent: {convo}"));
    assert!(
        !turns.is_empty(),
        "conversation.json should contain at least one turn"
    );

    let transcript_path = session_dir.join("transcript.md");
    assert!(
        transcript_path.exists(),
        "transcript.md should exist at {}",
        transcript_path.display()
    );
    let transcript = std::fs::read_to_string(&transcript_path).unwrap();
    assert!(
        !transcript.is_empty(),
        "transcript.md should not be empty"
    );
}
