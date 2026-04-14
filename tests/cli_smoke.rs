use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn cli_help_lists_subcommands() {
    Command::cargo_bin("conch")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("talk"))
        .stdout(contains("sessions"))
        .stdout(contains("export"));
}

#[test]
fn cli_no_args_shows_help_and_exits_nonzero() {
    Command::cargo_bin("conch")
        .unwrap()
        .assert()
        .failure();
}

#[test]
fn sessions_empty_prints_no_sessions_message() {
    let tmp = tempfile::TempDir::new().unwrap();

    Command::cargo_bin("conch")
        .unwrap()
        .env("HOME", tmp.path())
        .arg("sessions")
        .assert()
        .success()
        .stdout(contains("No sessions yet"));
}

#[test]
fn sessions_lists_existing_session_dirs() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sessions = tmp.path().join(".conch/sessions/2026-04-14-test");
    std::fs::create_dir_all(&sessions).unwrap();

    Command::cargo_bin("conch")
        .unwrap()
        .env("HOME", tmp.path())
        .arg("sessions")
        .assert()
        .success()
        .stdout(contains("2026-04-14-test"));
}
