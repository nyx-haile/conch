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

#[test]
fn config_from_env_reads_api_keys() {
    let home = PathBuf::from("/tmp/fake-home");
    let mut env = std::collections::HashMap::new();
    env.insert("ANTHROPIC_API_KEY".to_string(), "sk-test-123".to_string());
    env.insert("GITHUB_TOKEN".to_string(), "ghp-test-456".to_string());

    let config = Config::from_env_map(&home, &env);

    assert_eq!(config.anthropic_api_key(), Some("sk-test-123"));
    assert_eq!(config.github_token(), Some("ghp-test-456"));
}

#[test]
fn config_from_env_handles_missing_keys() {
    let home = PathBuf::from("/tmp/fake-home");
    let env = std::collections::HashMap::new();

    let config = Config::from_env_map(&home, &env);

    assert_eq!(config.anthropic_api_key(), None);
    assert_eq!(config.github_token(), None);
}
