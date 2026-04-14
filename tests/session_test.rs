use conch::session::SessionId;

#[test]
fn session_id_slug_from_topic() {
    let id = SessionId::new("2026-04-14", "howtowin.lol - a side project");
    assert_eq!(id.as_str(), "2026-04-14-howtowin-lol-a-side-project");
}

#[test]
fn session_id_slug_truncates_long_topics() {
    let long = "a".repeat(200);
    let id = SessionId::new("2026-04-14", &long);
    // Date prefix (11 chars) + max 60-char slug = 71 chars
    assert!(id.as_str().len() <= 71);
    assert!(id.as_str().starts_with("2026-04-14-"));
}

#[test]
fn session_id_slug_handles_github_url() {
    let id = SessionId::new("2026-04-14", "https://github.com/user/howtowin");
    assert_eq!(id.as_str(), "2026-04-14-github-com-user-howtowin");
}
