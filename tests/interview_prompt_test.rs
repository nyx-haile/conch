use conch::interview::prompt::compose_system_prompt;

#[test]
fn compose_uses_brief_and_defaults() {
    let brief = "# Brief\n\n## Summary\nA voice CLI.\n";
    let prompt = compose_system_prompt(brief, None, None);
    assert!(prompt.contains("You are Conch"));
    assert!(prompt.contains("A voice CLI."));
    assert!(prompt.contains("end_session"));
    assert!(prompt.contains("2-4 sentences"));
}

#[test]
fn compose_appends_brand_when_provided() {
    let brief = "# Brief\n";
    let brand = "Speak plainly. No jargon.";
    let prompt = compose_system_prompt(brief, Some(brand), None);
    assert!(prompt.contains("Speak plainly."));
    assert!(prompt.find("Speak plainly").unwrap() > prompt.find("You are Conch").unwrap());
}

#[test]
fn compose_appends_correction_memory_last() {
    let brief = "# Brief\n";
    let brand = "Speak plainly.";
    let memory = "Transcription memory:\n- heard \"pepper ex\" -> meant \"Pepper X\"";
    let prompt = compose_system_prompt(brief, Some(brand), Some(memory));
    assert!(prompt.contains("Pepper X"));
    assert!(
        prompt.find("Transcription memory").unwrap() > prompt.find("Speak plainly").unwrap(),
        "correction memory should come after brand guidance"
    );
}

#[test]
fn compose_ignores_blank_correction_memory() {
    let brief = "# Brief\n";
    let with_blank = compose_system_prompt(brief, None, Some("   \n"));
    let without = compose_system_prompt(brief, None, None);
    assert_eq!(with_blank, without);
}
