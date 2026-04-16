use conch::tts::{text::TextTts, TextToSpeech, TtsConfig};

#[tokio::test]
async fn text_tts_forwards_text_and_yields_no_audio() {
    let (tts, mut rx) = TextTts::new();
    let mut stream = tts.open_stream(&TtsConfig::default()).await.unwrap();
    stream.push_text("hello ").await.unwrap();
    stream.push_text("world").await.unwrap();
    stream.end_of_input().await.unwrap();

    assert_eq!(rx.recv().await.unwrap(), "hello ");
    assert_eq!(rx.recv().await.unwrap(), "world");
    assert!(stream.next_chunk().await.is_none());
}

#[tokio::test]
async fn text_tts_batch_returns_empty_pcm_per_text() {
    let (tts, _rx) = TextTts::new();
    let out = tts.synthesize_batch(&["foo", "bar"]).await.unwrap();
    assert_eq!(out.len(), 2);
    assert!(out.iter().all(|v| v.is_empty()));
}
