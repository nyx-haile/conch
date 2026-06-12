use conch::tts::local::LocalTts;
use std::path::PathBuf;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn local_tts_reports_missing_model() {
    let missing = PathBuf::from("/nonexistent/voice.onnx");
    let err = LocalTts::with_model(missing, "piper-tts").unwrap_err();
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("not found") && msg.contains("voice.onnx"),
        "expected missing-model error, got: {msg}"
    );
}

#[test]
fn local_tts_reports_missing_voice_config() {
    // Write an .onnx file but no .onnx.json sidecar.
    let tmp = tempfile::tempdir().unwrap();
    let model = tmp.path().join("voice.onnx");
    std::fs::write(&model, b"fake model").unwrap();
    let err = LocalTts::with_model(&model, "piper-tts").unwrap_err();
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("voice config") || msg.contains("voice.onnx.json"),
        "expected voice-config error, got: {msg}"
    );
}

#[test]
fn local_tts_expands_home_in_model_env() {
    let _lock = ENV_LOCK.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let model = tmp.path().join("voice.onnx");
    std::fs::write(&model, b"fake model").unwrap();
    std::fs::write(
        tmp.path().join("voice.onnx.json"),
        br#"{"audio":{"sample_rate":22050}}"#,
    )
    .unwrap();
    let _guard = EnvGuard::set("CONCH_PIPER_MODEL", "~/voice.onnx");

    let tts = LocalTts::from_env(tmp.path()).unwrap();
    assert_eq!(tts.sample_rate(), 22_050);
}

struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}
