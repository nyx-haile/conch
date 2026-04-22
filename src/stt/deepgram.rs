use crate::stt::{SpeechToText, SttConfig, SttStream, TranscriptEvent, Word};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub struct DeepgramStt {
    api_key: String,
    base_url: String,
}

impl DeepgramStt {
    pub fn new(api_key: impl Into<String>, base_url: &str) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn production(api_key: impl Into<String>) -> Self {
        Self::new(api_key, "wss://api.deepgram.com/v1/listen")
    }
}

#[async_trait]
impl SpeechToText for DeepgramStt {
    async fn open_stream(&self, config: &SttConfig) -> Result<Box<dyn SttStream>> {
        let sep = if self.base_url.contains('?') {
            '&'
        } else {
            '?'
        };
        let url = format!(
            "{}{}encoding=linear16&sample_rate={}&model=nova-2&interim_results=true&punctuate={}",
            self.base_url,
            sep,
            config.sample_rate.max(16_000),
            config.punctuate
        );
        let mut req = url
            .as_str()
            .into_client_request()
            .context("building ws request")?;
        req.headers_mut().insert(
            "Authorization",
            format!("Token {}", self.api_key)
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid api key for Authorization header"))?,
        );

        let (ws, _) = connect_async(req).await.context("connecting to deepgram")?;
        Ok(Box::new(DeepgramStream { ws }))
    }
}

pub struct DeepgramStream {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl std::fmt::Debug for DeepgramStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeepgramStream").finish_non_exhaustive()
    }
}

#[derive(Debug, Deserialize)]
struct DgResponse {
    channel: DgChannel,
    is_final: bool,
}

#[derive(Debug, Deserialize)]
struct DgChannel {
    alternatives: Vec<DgAlternative>,
}

#[derive(Debug, Deserialize)]
struct DgAlternative {
    transcript: String,
    #[serde(default)]
    words: Vec<DgWord>,
}

#[derive(Debug, Deserialize)]
struct DgWord {
    word: String,
    start: f64,
    end: f64,
}

#[async_trait]
impl SttStream for DeepgramStream {
    async fn send_frame(&mut self, pcm: &[i16]) -> Result<()> {
        let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
        self.ws
            .send(Message::Binary(bytes))
            .await
            .context("sending frame")
    }

    async fn end_of_utterance(&mut self) -> Result<()> {
        self.ws
            .send(Message::Text(r#"{"type":"CloseStream"}"#.into()))
            .await
            .context("sending CloseStream")
    }

    async fn next_event(&mut self) -> Option<TranscriptEvent> {
        while let Some(msg) = self.ws.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let parsed: Result<DgResponse, _> = serde_json::from_str(&text);
                    match parsed {
                        Ok(r) => {
                            let Some(alt) = r.channel.alternatives.into_iter().next() else {
                                continue;
                            };
                            if alt.transcript.is_empty() {
                                continue;
                            }
                            if r.is_final {
                                let words = alt
                                    .words
                                    .into_iter()
                                    .map(|w| Word {
                                        text: w.word,
                                        start_ms: (w.start * 1000.0) as u64,
                                        end_ms: (w.end * 1000.0) as u64,
                                    })
                                    .collect();
                                return Some(TranscriptEvent::Final {
                                    text: alt.transcript,
                                    words,
                                });
                            } else {
                                return Some(TranscriptEvent::Partial {
                                    text: alt.transcript,
                                    stability: 0.7,
                                });
                            }
                        }
                        Err(_) => continue,
                    }
                }
                Ok(Message::Close(_)) | Err(_) => return None,
                _ => continue,
            }
        }
        None
    }

    async fn close(&mut self) -> Result<()> {
        let _ = self.ws.close(None).await;
        Ok(())
    }
}
