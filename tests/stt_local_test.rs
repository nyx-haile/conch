use conch::stt::{local::LocalStt, SpeechToText, SttConfig, TranscriptEvent};
use std::path::PathBuf;

#[tokio::test]
async fn local_stt_errors_when_model_missing_and_download_disabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let stt = LocalStt::new(tmp.path().to_path_buf()).with_auto_download(false);
    // Guard against CI env having scripted finals set — we want the real
    // model-check path here.
    let guard = EnvGuard::unset("CONCH_TEST_SCRIPTED_STT_FINALS");

    let err = stt.open_stream(&SttConfig::default()).await.unwrap_err();
    let msg = format!("{err:#}").to_lowercase();
    assert!(
        msg.contains("parakeet") && msg.contains("missing"),
        "expected parakeet missing-model error, got {msg}"
    );

    drop(guard);
}

#[tokio::test]
async fn local_stt_emits_scripted_final_when_env_set() {
    // Verifies the test-path is still honored so `conch test` etc. run
    // without any real model on disk.
    let _guard = EnvGuard::set("CONCH_TEST_SCRIPTED_STT_FINALS", "hello world");
    let stt = LocalStt::new(PathBuf::from("/definitely/not/here"));
    let mut stream = stt.open_stream(&SttConfig::default()).await.unwrap();
    let ev = stream.next_event().await;
    match ev {
        Some(TranscriptEvent::Final { text, .. }) => assert_eq!(text, "hello world"),
        other => panic!("expected Final, got {other:?}"),
    }
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
    fn unset(key: &'static str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::remove_var(key);
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}
