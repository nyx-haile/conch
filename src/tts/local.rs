//! Piper-backed local TTS. Shells out to the `piper-tts` binary with
//! `--output-raw` and streams 16-bit mono PCM from stdout.
//!
//! Configuration:
//!   CONCH_PIPER_MODEL  path to the voice `.onnx` (a sibling `.onnx.json`
//!                      must exist — we parse `audio.sample_rate` from it).
//!   CONCH_PIPER_BIN    override the binary name (default: `piper-tts`).

use crate::tts::{TextToSpeech, TtsConfig, TtsStream};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

#[derive(Debug)]
pub struct LocalTts {
    model_path: PathBuf,
    binary: String,
    sample_rate: u32,
}

impl LocalTts {
    /// Build from explicit model path. Reads the sibling `.onnx.json` to
    /// determine the voice's sample rate.
    pub fn with_model(model_path: impl Into<PathBuf>, binary: impl Into<String>) -> Result<Self> {
        let model_path = model_path.into();
        if !model_path.exists() {
            return Err(anyhow!(
                "piper model not found at {}. Set CONCH_PIPER_MODEL or download a voice.",
                model_path.display()
            ));
        }
        let sample_rate = read_voice_sample_rate(&model_path)?;
        Ok(Self {
            model_path,
            binary: binary.into(),
            sample_rate,
        })
    }

    /// Resolve model path from env (`CONCH_PIPER_MODEL`) or the default
    /// `<home>/.conch/voices/en/en_US/lessac/medium/en_US-lessac-medium.onnx`.
    pub fn from_env(home: &Path) -> Result<Self> {
        let model_path = match std::env::var("CONCH_PIPER_MODEL") {
            Ok(s) => PathBuf::from(s),
            Err(_) => home.join(".conch/voices/en/en_US/lessac/medium/en_US-lessac-medium.onnx"),
        };
        let binary = std::env::var("CONCH_PIPER_BIN").unwrap_or_else(|_| "piper-tts".to_string());
        Self::with_model(model_path, binary)
    }
}

#[derive(Deserialize)]
struct VoiceConfig {
    audio: VoiceAudio,
}

#[derive(Deserialize)]
struct VoiceAudio {
    sample_rate: u32,
}

fn read_voice_sample_rate(model_path: &Path) -> Result<u32> {
    let json_path = {
        let mut p = model_path.to_path_buf();
        let file = p
            .file_name()
            .ok_or_else(|| anyhow!("model path has no filename: {}", model_path.display()))?
            .to_owned();
        let mut with_ext = file
            .into_string()
            .map_err(|_| anyhow!("non-utf8 model name"))?;
        with_ext.push_str(".json");
        p.set_file_name(with_ext);
        p
    };
    let bytes = std::fs::read(&json_path)
        .with_context(|| format!("reading voice config at {}", json_path.display()))?;
    let cfg: VoiceConfig = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing voice config {}", json_path.display()))?;
    Ok(cfg.audio.sample_rate)
}

#[async_trait]
impl TextToSpeech for LocalTts {
    async fn open_stream(&self, _config: &TtsConfig) -> Result<Box<dyn TtsStream>> {
        let mut child = Command::new(&self.binary)
            .arg("-m")
            .arg(&self.model_path)
            .arg("--output-raw")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("spawning {}", self.binary))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("piper stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("piper stdout unavailable"))?;
        Ok(Box::new(PiperStream {
            child: Some(child),
            stdin: Some(stdin),
            stdout: Some(BufReader::new(stdout)),
            sample_rate: self.sample_rate,
            done: false,
        }))
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

pub struct PiperStream {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    sample_rate: u32,
    done: bool,
}

impl std::fmt::Debug for PiperStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PiperStream")
            .field("sample_rate", &self.sample_rate)
            .field("done", &self.done)
            .finish_non_exhaustive()
    }
}

/// Read ~200ms of audio at a time (4KB @ 22050 Hz × 2 bytes ≈ 90ms; we go
/// slightly bigger for smoother playback without adding much latency).
const CHUNK_BYTES: usize = 8192;

#[async_trait]
impl TtsStream for PiperStream {
    async fn push_text(&mut self, chunk: &str) -> Result<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("piper stdin already closed"))?;
        stdin.write_all(chunk.as_bytes()).await?;
        Ok(())
    }

    async fn end_of_input(&mut self) -> Result<()> {
        // Dropping stdin signals EOF to piper so it flushes remaining audio.
        if let Some(mut stdin) = self.stdin.take() {
            stdin.write_all(b"\n").await.ok();
            stdin.shutdown().await.ok();
        }
        Ok(())
    }

    async fn next_chunk(&mut self) -> Option<Vec<i16>> {
        if self.done {
            return None;
        }
        let stdout = self.stdout.as_mut()?;
        let mut buf = vec![0u8; CHUNK_BYTES];
        match stdout.read(&mut buf).await {
            Ok(0) => {
                self.done = true;
                // Reap the child to avoid zombies.
                if let Some(mut ch) = self.child.take() {
                    let _ = ch.wait().await;
                }
                None
            }
            Ok(n) => {
                buf.truncate(n);
                // piper emits little-endian i16. If n is odd (shouldn't
                // happen but guard anyway), drop the trailing byte.
                let even = n & !1;
                let pcm: Vec<i16> = buf[..even]
                    .chunks_exact(2)
                    .map(|b| i16::from_le_bytes([b[0], b[1]]))
                    .collect();
                Some(pcm)
            }
            Err(_) => {
                self.done = true;
                None
            }
        }
    }

    async fn abort(&mut self) -> Result<()> {
        self.done = true;
        self.stdin.take();
        if let Some(mut ch) = self.child.take() {
            let _ = ch.kill().await;
            let _ = ch.wait().await;
        }
        Ok(())
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}
