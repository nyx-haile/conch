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
    model: String,
    endpointing_ms: u16,
    mip_opt_out: bool,
}

impl DeepgramStt {
    pub fn new(api_key: impl Into<String>, base_url: &str) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: "nova-3".to_string(),
            endpointing_ms: 350,
            mip_opt_out: true,
        }
    }

    pub fn production(api_key: impl Into<String>) -> Self {
        Self::new(api_key, "wss://api.deepgram.com/v1/listen")
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_endpointing_ms(mut self, endpointing_ms: u16) -> Self {
        self.endpointing_ms = endpointing_ms;
        self
    }

    pub fn with_mip_opt_out(mut self, enabled: bool) -> Self {
        self.mip_opt_out = enabled;
        self
    }
}

#[async_trait]
impl SpeechToText for DeepgramStt {
    async fn open_stream(&self, config: &SttConfig) -> Result<Box<dyn SttStream>> {
        let mut params = vec![
            ("encoding", "linear16".to_string()),
            ("sample_rate", config.sample_rate.max(16_000).to_string()),
            ("model", self.model.clone()),
            ("interim_results", "true".to_string()),
            ("endpointing", self.endpointing_ms.to_string()),
            ("punctuate", config.punctuate.to_string()),
            ("mip_opt_out", self.mip_opt_out.to_string()),
        ];
        if let Some(language) = &config.language {
            params.push(("language", language.clone()));
        }
        let sep = if self.base_url.contains('?') {
            '&'
        } else {
            '?'
        };
        let query = params
            .into_iter()
            .map(|(key, value)| format!("{key}={}", query_escape(&value)))
            .collect::<Vec<_>>()
            .join("&");
        let url = format!("{}{}{}", self.base_url, sep, query);
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
        Ok(Box::new(DeepgramStream {
            ws,
            close_sent: false,
            closed: false,
        }))
    }
}

pub struct DeepgramStream {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    close_sent: bool,
    closed: bool,
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
    #[serde(default)]
    speech_final: bool,
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
        if self.closed {
            return Ok(());
        }
        let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
        self.ws
            .send(Message::Binary(bytes))
            .await
            .context("sending frame")
    }

    async fn end_of_utterance(&mut self) -> Result<()> {
        if self.closed || self.close_sent {
            return Ok(());
        }
        match self
            .ws
            .send(Message::Text(r#"{"type":"CloseStream"}"#.into()))
            .await
        {
            Ok(()) => {
                self.close_sent = true;
                Ok(())
            }
            Err(e) if is_close_race(&e) => {
                self.closed = true;
                Ok(())
            }
            Err(e) => Err(e).context("sending CloseStream"),
        }
    }

    async fn next_event(&mut self) -> Option<TranscriptEvent> {
        while let Some(msg) = self.ws.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let parsed: Result<DgResponse, _> = serde_json::from_str(&text);
                    match parsed {
                        Ok(r) => {
                            let speech_final = r.speech_final;
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
                                if speech_final {
                                    tracing::debug!(target: "conch::stt::deepgram", "deepgram speech_final received");
                                }
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
                Ok(Message::Close(_)) => {
                    self.closed = true;
                    return None;
                }
                Err(_) => {
                    self.closed = true;
                    return None;
                }
                _ => continue,
            }
        }
        self.closed = true;
        None
    }

    async fn close(&mut self) -> Result<()> {
        self.closed = true;
        let _ = self.ws.close(None).await;
        Ok(())
    }
}

fn is_close_race(err: &tokio_tungstenite::tungstenite::Error) -> bool {
    matches!(
        err,
        tokio_tungstenite::tungstenite::Error::Protocol(
            tokio_tungstenite::tungstenite::error::ProtocolError::SendAfterClosing
        )
    )
}

fn query_escape(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
