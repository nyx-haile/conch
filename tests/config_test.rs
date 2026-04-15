use conch::config::Config;
use conch::provider::Provider;
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
fn config_from_env_reads_api_keys_and_provider() {
    let home = PathBuf::from("/tmp/fake-home");
    let mut env = std::collections::HashMap::new();
    env.insert("OPENROUTER_API_KEY".to_string(), "sk-or-test".to_string());
    env.insert("GITHUB_TOKEN".to_string(), "ghp-test-456".to_string());
    env.insert("CONCH_PROVIDER".to_string(), "anthropic".to_string());

    let config = Config::from_env_map(&home, &env).unwrap();

    assert_eq!(config.openrouter_api_key(), Some("sk-or-test"));
    assert_eq!(config.github_token(), Some("ghp-test-456"));
    assert_eq!(config.provider().unwrap(), Provider::Anthropic);
}

#[test]
fn config_from_env_errors_on_unknown_provider() {
    let home = PathBuf::from("/tmp/fake-home");
    let mut env = std::collections::HashMap::new();
    env.insert("CONCH_PROVIDER".to_string(), "openai".to_string());

    let err = Config::from_env_map(&home, &env).unwrap_err();
    assert!(err.to_string().contains("unknown provider"));
}

#[test]
fn config_without_provider_errors_when_provider_requested() {
    let home = PathBuf::from("/tmp/fake-home");
    let env = std::collections::HashMap::new();

    let config = Config::from_env_map(&home, &env).unwrap();

    assert_eq!(config.openrouter_api_key(), None);
    assert_eq!(config.github_token(), None);

    let err = config.provider().unwrap_err();
    assert!(err.to_string().contains("CONCH_PROVIDER"));
}
