use base64::Engine;
use conch::tts::elevenlabs::ElevenLabsTts;
use conch::tts::{TextToSpeech, TtsConfig};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

async fn run_mock_el() -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let url = format!("ws://127.0.0.1:{}", port);

    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();

        let mut captured: Vec<String> = Vec::new();
        while let Some(msg) = ws.next().await {
            match msg {
                Ok(Message::Text(t)) => {
                    captured.push(t.clone());
                    if t.contains("\"text\":\"\"") {
                        break;
                    }
                }
                _ => break,
            }
        }

        // Serve one mp3-encoded audio chunk from the test fixture.
        let mp3 = std::fs::read("tests/fixtures/tone_200ms_44100.mp3").unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&mp3);
        let payload = format!(r#"{{"audio":"{}","isFinal":true}}"#, b64);
        ws.send(Message::Text(payload)).await.unwrap();
        ws.close(None).await.unwrap();
        captured
    });

    (url, handle)
}

#[tokio::test]
async fn elevenlabs_pushes_text_and_yields_pcm() {
    let (url, server) = run_mock_el().await;
    let tts = ElevenLabsTts::new("el-key", &url, "voice-a");

    let mut stream = tts.open_stream(&TtsConfig::default()).await.unwrap();
    stream.push_text("hello ").await.unwrap();
    stream.push_text("world").await.unwrap();
    stream.end_of_input().await.unwrap();

    let chunk = stream.next_chunk().await.unwrap();
    assert!(!chunk.is_empty(), "expected decoded PCM");

    let captured = server.await.unwrap();
    assert!(captured.iter().any(|m| m.contains("hello ")));
    assert!(captured.iter().any(|m| m.contains("world")));
    assert!(captured.iter().any(|m| m.contains("\"text\":\"\"")));
}
