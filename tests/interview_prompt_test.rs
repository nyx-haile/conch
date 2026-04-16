use conch::interview::prompt::compose_system_prompt;

#[test]
fn compose_uses_brief_and_defaults() {
    let brief = "# Brief\n\n## Summary\nA voice CLI.\n";
    let prompt = compose_system_prompt(brief, None);
    assert!(prompt.contains("You are Conch"));
    assert!(prompt.contains("A voice CLI."));
    assert!(prompt.contains("end_session"));
    assert!(prompt.contains("2-4 sentences"));
}

#[test]
fn compose_appends_brand_when_provided() {
    let brief = "# Brief\n";
    let brand = "Speak plainly. No jargon.";
    let prompt = compose_system_prompt(brief, Some(brand));
    assert!(prompt.contains("Speak plainly."));
    assert!(prompt.find("Speak plainly").unwrap() > prompt.find("You are Conch").unwrap());
}
