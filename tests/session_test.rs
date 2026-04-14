use conch::session::{Session, SessionId};
use tempfile::TempDir;

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

#[test]
fn session_create_makes_directory() {
    let tmp = TempDir::new().unwrap();
    let sessions_root = tmp.path().join("sessions");

    let session = Session::create(&sessions_root, "2026-04-14", "test-project").unwrap();

    assert!(session.directory().exists());
    assert!(session.directory().is_dir());
    assert_eq!(
        session.directory(),
        sessions_root.join("2026-04-14-test-project")
    );
}

#[test]
fn session_paths_point_inside_directory() {
    let tmp = TempDir::new().unwrap();
    let sessions_root = tmp.path().join("sessions");
    let session = Session::create(&sessions_root, "2026-04-14", "test").unwrap();

    assert_eq!(session.brief_path(), session.directory().join("brief.md"));
    assert_eq!(session.raw_audio_path(), session.directory().join("raw_audio.wav"));
    assert_eq!(session.transcript_path(), session.directory().join("transcript.md"));
    assert_eq!(session.edited_path(), session.directory().join("edited.md"));
}
