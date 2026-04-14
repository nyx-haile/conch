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
