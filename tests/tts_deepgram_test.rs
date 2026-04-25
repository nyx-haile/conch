use base64::Engine;
use conch::tts::deepgram::DeepgramTts;
use conch::tts::{TextToSpeech, TtsConfig};
use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Default)]
struct CapturedHandshake {
    auth: Option<String>,
    query: Option<String>,
}

async fn run_mock_deepgram_tts() -> (
    String,
    tokio::task::JoinHandle<(CapturedHandshake, Vec<String>)>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let url = format!("ws://127.0.0.1:{}/v1/speak", port);

    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let captured = Arc::new(Mutex::new(CapturedHandshake::default()));
        let captured_for_hdr = captured.clone();

        let mut ws = tokio_tungstenite::accept_hdr_async(
            stream,
            move |req: &http::Request<()>,
                  resp: http::Response<()>|
                  -> Result<http::Response<()>, http::Response<Option<String>>> {
                let mut guard = captured_for_hdr.lock().unwrap();
                guard.auth = req
                    .headers()
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                guard.query = req.uri().query().map(|s| s.to_string());
                Ok(resp)
            },
        )
        .await
        .unwrap();

        let mut messages = Vec::new();
        while let Some(msg) = ws.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let should_close = text.contains(r#""type":"Close""#);
                    messages.push(text);
                    if should_close {
                        break;
                    }
                }
                _ => break,
            }
        }

        ws.send(Message::Text(
            r#"{"type":"Metadata","request_id":"dg-tts-req"}"#.into(),
        ))
        .await
        .unwrap();
        let pcm = [0i16, i16::MAX, i16::MIN + 1];
        let bytes: Vec<u8> = pcm.iter().flat_map(|sample| sample.to_le_bytes()).collect();
        ws.send(Message::Binary(bytes)).await.unwrap();
        ws.close(None).await.unwrap();

        let captured = Arc::try_unwrap(captured).unwrap().into_inner().unwrap();
        (captured, messages)
    });

    (url, handle)
}

#[tokio::test]
async fn deepgram_tts_sends_speak_close_and_yields_linear16_pcm() {
    let (url, server) = run_mock_deepgram_tts().await;
    let tts = DeepgramTts::new("dg-test-key", &url, "aura-2-thalia-en");

    let mut stream = tts.open_stream(&TtsConfig::default()).await.unwrap();
    assert_eq!(stream.sample_rate(), 24_000);
    stream.push_text("hello from conch").await.unwrap();
    stream.end_of_input().await.unwrap();

    let chunk = stream.next_chunk().await.unwrap();
    assert_eq!(chunk, vec![0, i16::MAX, i16::MIN + 1]);
    assert!(stream.next_chunk().await.is_none());

    let (captured, messages) = server.await.unwrap();
    assert_eq!(captured.auth.as_deref(), Some("Token dg-test-key"));
    let query = captured.query.unwrap();
    assert!(query.contains("encoding=linear16"), "query={query}");
    assert!(query.contains("sample_rate=24000"), "query={query}");
    assert!(query.contains("model=aura-2-thalia-en"), "query={query}");
    assert!(query.contains("mip_opt_out=true"), "query={query}");
    assert!(messages.iter().any(|m| m.contains("hello from conch")));
    assert!(messages.iter().any(|m| m.contains(r#""type":"Close""#)));
}

#[tokio::test]
async fn deepgram_tts_config_voice_overrides_default_model() {
    let (url, server) = run_mock_deepgram_tts().await;
    let tts = DeepgramTts::new("dg-test-key", &url, "aura-2-thalia-en");

    let mut stream = tts
        .open_stream(&TtsConfig {
            voice_id: Some("aura-2-asteria-en".to_string()),
        })
        .await
        .unwrap();
    stream.push_text("hello").await.unwrap();
    stream.end_of_input().await.unwrap();
    while stream.next_chunk().await.is_some() {}

    let (captured, _) = server.await.unwrap();
    let query = captured.query.unwrap();
    assert!(query.contains("model=aura-2-asteria-en"), "query={query}");
}

#[test]
fn deepgram_tts_audio_payloads_are_base64_safe_for_realtime_contract() {
    let pcm = [1i16, -2i16];
    let bytes: Vec<u8> = pcm.iter().flat_map(|sample| sample.to_le_bytes()).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    assert_eq!(encoded, "AQD+/w==");
}
