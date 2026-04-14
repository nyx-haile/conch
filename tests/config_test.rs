use conch::config::Config;
use std::path::PathBuf;

#[test]
fn config_with_home_override_returns_expected_paths() {
    let home = PathBuf::from("/tmp/fake-home");
    let config = Config::with_home(&home);

    assert_eq!(config.conch_dir(), home.join(".conch"));
    assert_eq!(config.sessions_dir(), home.join(".conch/sessions"));
    assert_eq!(config.brand_file(), home.join(".conch/brand.md"));
}
