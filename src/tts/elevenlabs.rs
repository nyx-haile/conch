use crate::audio::convert::decode_mp3_to_pcm;
use crate::tts::{TextToSpeech, TtsConfig, TtsStream};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub struct ElevenLabsTts {
    api_key: String,
    base_url: String,
    voice_id: String,
}

impl ElevenLabsTts {
    pub fn new(api_key: impl Into<String>, base_url: &str, voice_id: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.trim_end_matches('/').to_string(),
            voice_id: voice_id.into(),
        }
    }

    pub fn production(api_key: impl Into<String>, voice_id: impl Into<String>) -> Self {
        Self::new(api_key, "wss://api.elevenlabs.io", voice_id)
    }

    fn stream_url(&self, _config: &TtsConfig) -> String {
        if self.base_url.starts_with("ws://127.0.0.1")
            || self.base_url.starts_with("ws://localhost")
        {
            self.base_url.clone()
        } else {
            format!(
                "{}/v1/text-to-speech/{}/stream-input?model_id=eleven_turbo_v2",
                self.base_url, self.voice_id
            )
        }
    }
}

#[async_trait]
impl TextToSpeech for ElevenLabsTts {
    async fn open_stream(&self, config: &TtsConfig) -> Result<Box<dyn TtsStream>> {
        let url = self.stream_url(config);
        let mut req = url
            .as_str()
            .into_client_request()
            .context("building ws request")?;
        req.headers_mut().insert(
            "xi-api-key",
            self.api_key
                .parse()
                .map_err(|_| anyhow!("invalid api key for xi-api-key header"))?,
        );
        let (ws, _) = connect_async(req)
            .await
            .context("connecting to elevenlabs")?;
        Ok(Box::new(ElevenLabsStream { ws, done: false }))
    }

    async fn synthesize_batch(&self, texts: &[&str]) -> Result<Vec<Vec<i16>>> {
        let mut out = Vec::with_capacity(texts.len());
        for t in texts {
            let mut stream = self.open_stream(&TtsConfig::default()).await?;
            stream.push_text(t).await?;
            stream.end_of_input().await?;
            let mut pcm = Vec::new();
            while let Some(chunk) = stream.next_chunk().await {
                pcm.extend(chunk);
            }
            out.push(pcm);
        }
        Ok(out)
    }
}

pub struct ElevenLabsStream {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    done: bool,
}

impl std::fmt::Debug for ElevenLabsStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElevenLabsStream").finish_non_exhaustive()
    }
}

#[derive(Debug, Deserialize)]
struct AudioMessage {
    audio: Option<String>,
    #[serde(default, rename = "isFinal")]
    is_final: bool,
}

#[async_trait]
impl TtsStream for ElevenLabsStream {
    async fn push_text(&mut self, chunk: &str) -> Result<()> {
        let payload = serde_json::json!({ "text": chunk });
        self.ws
            .send(Message::Text(payload.to_string()))
            .await
            .context("push_text")
    }

    async fn end_of_input(&mut self) -> Result<()> {
        let payload = serde_json::json!({ "text": "" });
        self.ws
            .send(Message::Text(payload.to_string()))
            .await
            .context("end_of_input")
    }

    async fn next_chunk(&mut self) -> Option<Vec<i16>> {
        if self.done {
            return None;
        }
        while let Some(msg) = self.ws.next().await {
            match msg {
                Ok(Message::Text(t)) => {
                    let parsed: AudioMessage = match serde_json::from_str(&t) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    if parsed.is_final {
                        self.done = true;
                    }
                    if let Some(b64) = parsed.audio {
                        let bytes =
                            base64::engine::general_purpose::STANDARD.decode(&b64).ok()?;
                        let (pcm, _rate) = decode_mp3_to_pcm(&bytes).ok()?;
                        if !pcm.is_empty() {
                            return Some(pcm);
                        }
                    }
                    if self.done {
                        return None;
                    }
                }
                Ok(Message::Close(_)) | Err(_) => {
                    self.done = true;
                    return None;
                }
                _ => continue,
            }
        }
        None
    }

    async fn abort(&mut self) -> Result<()> {
        self.done = true;
        let _ = self.ws.close(None).await;
        Ok(())
    }

    fn sample_rate(&self) -> u32 {
        // ElevenLabs stream default is mp3_44100_128. If we ever configure
        // output_format differently (e.g. in stream_url), update here.
        44_100
    }
}
