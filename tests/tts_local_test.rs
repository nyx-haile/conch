use conch::tts::local::LocalTts;
use std::path::PathBuf;

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
