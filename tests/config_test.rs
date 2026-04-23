use conch::config::Config;
use conch::provider::Provider;
use std::path::PathBuf;
use tempfile::tempdir;

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
    env.insert("ANTHROPIC_API_KEY".to_string(), "sk-ant-test".to_string());
    env.insert("OPENAI_API_KEY".to_string(), "sk-openai-test".to_string());
    env.insert("GITHUB_TOKEN".to_string(), "ghp-test-456".to_string());
    env.insert("CONCH_PROVIDER".to_string(), "anthropic".to_string());

    let config = Config::from_env_map(&home, &env).unwrap();

    assert_eq!(config.openrouter_api_key(), Some("sk-or-test"));
    assert_eq!(config.anthropic_api_key(), Some("sk-ant-test"));
    assert_eq!(config.openai_api_key(), Some("sk-openai-test"));
    assert_eq!(config.github_token(), Some("ghp-test-456"));
    assert_eq!(config.provider().unwrap(), Provider::Anthropic);
}

#[test]
fn config_from_env_errors_on_unknown_provider() {
    let home = PathBuf::from("/tmp/fake-home");
    let mut env = std::collections::HashMap::new();
    env.insert("CONCH_PROVIDER".to_string(), "mistral".to_string());

    let err = Config::from_env_map(&home, &env).unwrap_err();
    assert!(err.to_string().contains("unknown provider"));
}

#[test]
fn config_from_env_reads_openai_key_from_file() {
    let home = tempdir().unwrap();
    let secret_path = home.path().join("build/solving/openai");
    std::fs::create_dir_all(secret_path.parent().unwrap()).unwrap();
    std::fs::write(&secret_path, " sk-openai-file \n").unwrap();

    let mut env = std::collections::HashMap::new();
    env.insert("CONCH_PROVIDER".to_string(), "openai".to_string());
    env.insert(
        "OPENAI_API_KEY_FILE".to_string(),
        "~/build/solving/openai".to_string(),
    );

    let config = Config::from_env_map(home.path(), &env).unwrap();

    assert_eq!(
        config.resolve_openai_api_key().unwrap().as_deref(),
        Some("sk-openai-file")
    );
    assert_eq!(config.provider().unwrap(), Provider::OpenAI);
}

#[test]
fn config_without_provider_errors_when_provider_requested() {
    let home = PathBuf::from("/tmp/fake-home");
    let env = std::collections::HashMap::new();

    let config = Config::from_env_map(&home, &env).unwrap();

    assert_eq!(config.openrouter_api_key(), None);
    assert_eq!(config.anthropic_api_key(), None);
    assert_eq!(config.openai_api_key(), None);
    assert_eq!(config.github_token(), None);

    let err = config.provider().unwrap_err();
    assert!(err.to_string().contains("CONCH_PROVIDER"));
}

use conch::config::{SttBackend, TtsBackend};

#[test]
fn config_from_env_reads_audio_keys_and_backends() {
    let home = PathBuf::from("/tmp/fake-home");
    let mut env = std::collections::HashMap::new();
    env.insert("DEEPGRAM_API_KEY".to_string(), "dg-test".to_string());
    env.insert("ELEVENLABS_API_KEY".to_string(), "el-test".to_string());
    env.insert("CONCH_STT".to_string(), "deepgram".to_string());
    env.insert("CONCH_TTS".to_string(), "text".to_string());
    env.insert("CONCH_PROVIDER".to_string(), "anthropic".to_string());

    let config = Config::from_env_map(&home, &env).unwrap();

    assert_eq!(config.deepgram_api_key(), Some("dg-test"));
    assert_eq!(config.elevenlabs_api_key(), Some("el-test"));
    assert_eq!(config.stt_backend(), SttBackend::Deepgram);
    assert_eq!(config.tts_backend(), TtsBackend::Text);
}

#[test]
fn config_defaults_backends_when_env_missing() {
    let home = PathBuf::from("/tmp/fake-home");
    let env = std::collections::HashMap::new();
    let config = Config::from_env_map(&home, &env).unwrap();
    assert_eq!(config.stt_backend(), SttBackend::Deepgram);
    assert_eq!(config.tts_backend(), TtsBackend::Local);
}

#[test]
fn config_errors_on_unknown_stt_backend() {
    let home = PathBuf::from("/tmp/fake-home");
    let mut env = std::collections::HashMap::new();
    env.insert("CONCH_STT".to_string(), "mumble".to_string());
    let err = Config::from_env_map(&home, &env).unwrap_err();
    assert!(err.to_string().contains("unknown STT backend"));
}
