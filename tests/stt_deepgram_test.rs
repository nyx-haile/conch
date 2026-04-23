use conch::stt::deepgram::DeepgramStt;
use conch::stt::{SpeechToText, SttConfig, TranscriptEvent};
use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

/// Spawns a tokio-tungstenite server that accepts one WS connection, records
/// the Authorization header, echoes back two JSON events, and closes.
async fn run_mock_server() -> (String, tokio::task::JoinHandle<Option<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let url = format!("ws://127.0.0.1:{}/v1/listen", port);

    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();

        let captured_auth: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let auth_clone = captured_auth.clone();

        let mut ws = tokio_tungstenite::accept_hdr_async(
            stream,
            move |req: &http::Request<()>,
                  resp: http::Response<()>|
                  -> Result<http::Response<()>, http::Response<Option<String>>> {
                let auth = req
                    .headers()
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                *auth_clone.lock().unwrap() = auth;
                Ok(resp)
            },
        )
        .await
        .unwrap();

        // Wait for at least one binary frame.
        while let Some(msg) = ws.next().await {
            if let Ok(Message::Binary(_)) = msg {
                break;
            }
        }

        ws.send(Message::Text(
            r#"{"channel":{"alternatives":[{"transcript":"hel","words":[]}]},"is_final":false,"speech_final":false}"#.into(),
        ))
        .await
        .unwrap();
        ws.send(Message::Text(
            r#"{"channel":{"alternatives":[{"transcript":"hello","words":[{"word":"hello","start":0.0,"end":0.5}]}]},"is_final":true,"speech_final":true}"#.into(),
        ))
        .await
        .unwrap();
        ws.close(None).await.unwrap();

        Arc::try_unwrap(captured_auth)
            .unwrap()
            .into_inner()
            .unwrap()
    });
    (url, handle)
}

#[tokio::test]
async fn deepgram_auth_and_events() {
    let (url, server) = run_mock_server().await;
    let stt = DeepgramStt::new("dg-test-key", &url);
    let mut stream = stt
        .open_stream(&SttConfig {
            sample_rate: 16_000,
            language: None,
            punctuate: true,
        })
        .await
        .unwrap();

    stream.send_frame(&[0i16; 320]).await.unwrap();
    stream.end_of_utterance().await.unwrap();

    let mut events: Vec<TranscriptEvent> = Vec::new();
    while let Some(e) = stream.next_event().await {
        events.push(e);
    }
    stream.close().await.unwrap();

    let auth = server.await.unwrap();
    assert_eq!(auth.as_deref(), Some("Token dg-test-key"));

    assert!(
        matches!(events[0], TranscriptEvent::Partial { ref text, .. } if text == "hel"),
        "expected Partial with 'hel', got {:?}",
        events[0]
    );
    assert!(
        matches!(events[1], TranscriptEvent::Final { ref text, .. } if text == "hello"),
        "expected Final with 'hello', got {:?}",
        events[1]
    );
}

#[tokio::test]
async fn deepgram_end_of_utterance_is_ok_after_server_closes() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let url = format!("ws://127.0.0.1:{}/v1/listen", port);

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        while let Some(msg) = ws.next().await {
            if let Ok(Message::Binary(_)) = msg {
                ws.close(None).await.unwrap();
                break;
            }
        }
    });

    let stt = DeepgramStt::new("dg-test-key", &url);
    let mut stream = stt
        .open_stream(&SttConfig {
            sample_rate: 16_000,
            language: None,
            punctuate: true,
        })
        .await
        .unwrap();

    stream.send_frame(&[0i16; 320]).await.unwrap();
    assert!(
        stream.next_event().await.is_none(),
        "server closed the stream"
    );
    stream.end_of_utterance().await.unwrap();
    stream.close().await.unwrap();

    server.await.unwrap();
}
