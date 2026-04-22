use conch::interview::intent::{detect_end_command, end_session_tool};

#[test]
fn detects_end_phrases_case_insensitively() {
    assert!(detect_end_command("okay, that's a wrap for today"));
    assert!(detect_end_command("That's A Wrap"));
    assert!(detect_end_command("let's end the session here"));
    assert!(detect_end_command("end interview"));
    assert!(detect_end_command("we're done, thanks"));
}

#[test]
fn ignores_unrelated_phrases() {
    assert!(!detect_end_command("that was a fun interview"));
    assert!(!detect_end_command(
        "so we have a wrapper around the client"
    ));
    assert!(!detect_end_command(""));
}

#[test]
fn end_session_tool_schema_matches_contract() {
    let def = end_session_tool();
    assert_eq!(def.function.name, "end_session");
    let props = def.function.parameters["properties"].as_object().unwrap();
    assert!(props.contains_key("reason"));
    let required = def.function.parameters["required"].as_array().unwrap();
    assert!(required.iter().any(|v| v == "reason"));
}
