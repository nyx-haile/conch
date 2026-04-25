use crate::tts::{TextToSpeech, TtsConfig, TtsStream};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub const DEFAULT_DEEPGRAM_TTS_MODEL: &str = "aura-2-thalia-en";
pub const DEFAULT_DEEPGRAM_TTS_SAMPLE_RATE: u32 = 24_000;

pub struct DeepgramTts {
    api_key: String,
    base_url: String,
    model: String,
    sample_rate: u32,
    mip_opt_out: bool,
}

impl DeepgramTts {
    pub fn new(api_key: impl Into<String>, base_url: &str, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.into(),
            sample_rate: DEFAULT_DEEPGRAM_TTS_SAMPLE_RATE,
            mip_opt_out: true,
        }
    }

    pub fn production(api_key: impl Into<String>) -> Self {
        Self::new(
            api_key,
            "wss://api.deepgram.com/v1/speak",
            DEFAULT_DEEPGRAM_TTS_MODEL,
        )
    }

    pub fn production_with_model(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new(api_key, "wss://api.deepgram.com/v1/speak", model)
    }

    pub fn with_sample_rate(mut self, sample_rate: u32) -> Self {
        self.sample_rate = sample_rate;
        self
    }

    pub fn with_mip_opt_out(mut self, enabled: bool) -> Self {
        self.mip_opt_out = enabled;
        self
    }

    fn stream_url(&self, config: &TtsConfig) -> String {
        let model = config.voice_id.as_deref().unwrap_or(&self.model);
        let sep = if self.base_url.contains('?') {
            '&'
        } else {
            '?'
        };
        format!(
            "{}{}encoding=linear16&sample_rate={}&model={}&mip_opt_out={}",
            self.base_url,
            sep,
            self.sample_rate,
            query_escape(model),
            self.mip_opt_out
        )
    }
}

#[async_trait]
impl TextToSpeech for DeepgramTts {
    async fn open_stream(&self, config: &TtsConfig) -> Result<Box<dyn TtsStream>> {
        let url = self.stream_url(config);
        let mut req = url
            .as_str()
            .into_client_request()
            .context("building deepgram tts ws request")?;
        req.headers_mut().insert(
            "Authorization",
            format!("Token {}", self.api_key)
                .parse()
                .map_err(|_| anyhow!("invalid api key for Authorization header"))?,
        );
        let (ws, _) = connect_async(req)
            .await
            .context("connecting to deepgram tts")?;
        Ok(Box::new(DeepgramTtsStream {
            ws,
            done: false,
            close_sent: false,
            sample_rate: self.sample_rate,
            request_id: None,
        }))
    }

    async fn synthesize_batch(&self, texts: &[&str]) -> Result<Vec<Vec<i16>>> {
        let mut out = Vec::with_capacity(texts.len());
        for text in texts {
            let mut stream = self.open_stream(&TtsConfig::default()).await?;
            stream.push_text(text).await?;
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

pub struct DeepgramTtsStream {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    done: bool,
    close_sent: bool,
    sample_rate: u32,
    request_id: Option<String>,
}

impl std::fmt::Debug for DeepgramTtsStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeepgramTtsStream")
            .field("done", &self.done)
            .field("close_sent", &self.close_sent)
            .field("sample_rate", &self.sample_rate)
            .field("request_id", &self.request_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum DeepgramTtsControl {
    #[serde(rename = "Metadata")]
    Metadata { request_id: Option<String> },
    #[serde(rename = "Flushed")]
    Flushed { sequence_id: Option<u64> },
    #[serde(rename = "Cleared")]
    Cleared { sequence_id: Option<u64> },
    #[serde(rename = "Warning")]
    Warning {
        code: Option<String>,
        description: Option<String>,
    },
    #[serde(other)]
    Other,
}

#[async_trait]
impl TtsStream for DeepgramTtsStream {
    async fn push_text(&mut self, chunk: &str) -> Result<()> {
        if self.done || chunk.trim().is_empty() {
            return Ok(());
        }
        let payload = serde_json::json!({ "type": "Speak", "text": chunk });
        self.ws
            .send(Message::Text(payload.to_string()))
            .await
            .context("deepgram tts speak")
    }

    async fn end_of_input(&mut self) -> Result<()> {
        if self.done || self.close_sent {
            return Ok(());
        }
        let payload = serde_json::json!({ "type": "Close" });
        self.ws
            .send(Message::Text(payload.to_string()))
            .await
            .context("deepgram tts close")?;
        self.close_sent = true;
        Ok(())
    }

    async fn next_chunk(&mut self) -> Option<Vec<i16>> {
        if self.done {
            return None;
        }
        while let Some(msg) = self.ws.next().await {
            match msg {
                Ok(Message::Binary(bytes)) => {
                    let pcm = linear16_le_to_pcm(&bytes);
                    if !pcm.is_empty() {
                        return Some(pcm);
                    }
                }
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<DeepgramTtsControl>(&text) {
                        Ok(DeepgramTtsControl::Metadata { request_id }) => {
                            self.request_id = request_id;
                        }
                        Ok(DeepgramTtsControl::Warning { code, description }) => {
                            tracing::warn!(
                                target: "conch::tts::deepgram",
                                code = ?code,
                                description = ?description,
                                "deepgram tts warning"
                            );
                        }
                        Ok(DeepgramTtsControl::Flushed { sequence_id }) => {
                            let _ = sequence_id;
                        }
                        Ok(DeepgramTtsControl::Cleared { sequence_id }) => {
                            let _ = sequence_id;
                            self.done = true;
                            return None;
                        }
                        Ok(DeepgramTtsControl::Other) | Err(_) => continue,
                    }
                }
                Ok(Message::Close(_)) | Err(_) => {
                    self.done = true;
                    return None;
                }
                _ => continue,
            }
        }
        self.done = true;
        None
    }

    async fn abort(&mut self) -> Result<()> {
        if !self.done {
            let payload = serde_json::json!({ "type": "Clear" });
            let _ = self.ws.send(Message::Text(payload.to_string())).await;
        }
        self.done = true;
        let _ = self.ws.close(None).await;
        Ok(())
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

fn linear16_le_to_pcm(bytes: &[u8]) -> Vec<i16> {
    bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect()
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

#[cfg(test)]
mod tests {
    use super::linear16_le_to_pcm;

    #[test]
    fn linear16_decoder_ignores_trailing_byte() {
        assert_eq!(linear16_le_to_pcm(&[0, 0, 255, 127, 1]), vec![0, i16::MAX]);
    }
}
