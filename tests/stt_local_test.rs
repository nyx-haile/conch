use conch::stt::{local::LocalStt, SpeechToText, SttConfig};

#[tokio::test]
async fn local_stt_returns_not_implemented_error() {
    let stt = LocalStt::new();
    let err = stt.open_stream(&SttConfig::default()).await.unwrap_err();
    assert!(err.to_string().to_lowercase().contains("not implemented"));
}
