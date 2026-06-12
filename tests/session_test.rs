use conch::session::{Session, SessionId};
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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

#[cfg(unix)]
#[test]
fn session_create_uses_private_directory_permissions() {
    let tmp = TempDir::new().unwrap();
    let sessions_root = tmp.path().join(".conch/sessions");

    let session = Session::create(&sessions_root, "2026-04-14", "private").unwrap();

    let conch_mode = std::fs::metadata(tmp.path().join(".conch"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    let sessions_mode = std::fs::metadata(&sessions_root)
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    let session_mode = std::fs::metadata(session.directory())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;

    assert_eq!(conch_mode & 0o077, 0);
    assert_eq!(sessions_mode & 0o077, 0);
    assert_eq!(session_mode & 0o077, 0);
}

#[test]
fn session_paths_point_inside_directory() {
    let tmp = TempDir::new().unwrap();
    let sessions_root = tmp.path().join("sessions");
    let session = Session::create(&sessions_root, "2026-04-14", "test").unwrap();

    assert_eq!(session.brief_path(), session.directory().join("brief.md"));
    assert_eq!(
        session.raw_audio_path(),
        session.directory().join("raw_audio.wav")
    );
    assert_eq!(
        session.transcript_path(),
        session.directory().join("transcript.md")
    );
    assert_eq!(session.edited_path(), session.directory().join("edited.md"));
}

#[test]
fn list_sessions_returns_existing_dirs_sorted_desc() {
    let tmp = TempDir::new().unwrap();
    let sessions_root = tmp.path().join("sessions");
    Session::create(&sessions_root, "2026-04-12", "old").unwrap();
    Session::create(&sessions_root, "2026-04-14", "new").unwrap();
    Session::create(&sessions_root, "2026-04-13", "mid").unwrap();

    let listed = conch::session::list_sessions(&sessions_root).unwrap();
    let ids: Vec<&str> = listed.iter().map(|s| s.as_str()).collect();

    assert_eq!(
        ids,
        vec!["2026-04-14-new", "2026-04-13-mid", "2026-04-12-old",]
    );
}

#[test]
fn list_sessions_returns_empty_when_no_dir() {
    let tmp = TempDir::new().unwrap();
    let sessions_root = tmp.path().join("does-not-exist");

    let listed = conch::session::list_sessions(&sessions_root).unwrap();
    assert!(listed.is_empty());
}
