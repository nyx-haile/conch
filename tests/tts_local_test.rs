use conch::tts::{local::LocalTts, TextToSpeech, TtsConfig};

#[tokio::test]
async fn local_tts_returns_not_implemented() {
    let tts = LocalTts::new();
    let err = tts.open_stream(&TtsConfig::default()).await.unwrap_err();
    assert!(err.to_string().to_lowercase().contains("not implemented"));
}
