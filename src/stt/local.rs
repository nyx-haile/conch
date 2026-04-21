//! Local STT backend powered by `parakeet-rs` (NVIDIA Nemotron streaming
//! 0.6B int8 via ONNX Runtime).
//!
//! The streaming loop shape — 560ms f32 windows fed to
//! `Nemotron::transcribe_chunk`, `reset()` between utterances — follows the
//! reference design in pepper-x's `pepperx-asr` crate (MIT, Jesse Vincent).
//! See LICENSES/pepper-x-MIT.txt.

use crate::stt::{SpeechToText, SttConfig, SttStream, TranscriptEvent};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

/// 560 ms at 16 kHz. Parakeet's streaming chunk size.
const CHUNK_SAMPLES: usize = 8960;
const PROGRESS_REPORT_PERCENT_STEP: u64 = 5;
const PROGRESS_REPORT_BYTES_STEP: u64 = 8 * 1024 * 1024;

/// Files the Nemotron int8 bundle expects on disk.
const MODEL_FILES: &[(&str, &str)] = &[
    (
        "encoder.onnx",
        "https://huggingface.co/smcleod/nemotron-speech-streaming-en-0.6b-int8/resolve/main/encoder.onnx",
    ),
    (
        "decoder_joint.onnx",
        "https://huggingface.co/smcleod/nemotron-speech-streaming-en-0.6b-int8/resolve/main/decoder_joint.onnx",
    ),
    (
        "tokenizer.model",
        "https://huggingface.co/smcleod/nemotron-speech-streaming-en-0.6b-int8/resolve/main/tokenizer.model",
    ),
];

pub struct LocalStt {
    model_dir: PathBuf,
    auto_download: bool,
}

impl LocalStt {
    pub fn new(model_dir: PathBuf) -> Self {
        let auto_download = std::env::var_os("CONCH_PARAKEET_NO_DOWNLOAD").is_none();
        Self {
            model_dir,
            auto_download,
        }
    }

    pub fn with_auto_download(mut self, enable: bool) -> Self {
        self.auto_download = enable;
        self
    }

    pub fn model_dir(&self) -> &Path {
        &self.model_dir
    }

    /// Ensure the model bundle is present on disk. Downloads missing files
    /// from the pinned HF repo when `auto_download` is enabled. Call this
    /// before starting the TUI so the first utterance doesn't block for
    /// several minutes on a cold machine.
    pub async fn prepare(&self) -> Result<()> {
        if std::env::var_os("CONCH_TEST_SCRIPTED_STT_FINALS").is_some() {
            return Ok(());
        }
        ensure_model(&self.model_dir, self.auto_download).await
    }
}

#[async_trait]
impl SpeechToText for LocalStt {
    async fn open_stream(&self, _config: &SttConfig) -> Result<Box<dyn SttStream>> {
        if let Ok(script) = std::env::var("CONCH_TEST_SCRIPTED_STT_FINALS") {
            let finals: Vec<String> = script.split('|').map(|s| s.to_string()).collect();
            return Ok(Box::new(ScriptedLocalStream {
                finals: finals.into_iter().collect(),
            }));
        }

        ensure_model(&self.model_dir, self.auto_download)
            .await
            .with_context(|| format!("preparing parakeet model at {}", self.model_dir.display()))?;

        let model_dir = self.model_dir.clone();
        let (cmd_tx, cmd_rx) = mpsc::channel::<SttCommand>(64);
        let (event_tx, event_rx) = mpsc::channel::<TranscriptEvent>(64);

        tokio::task::spawn_blocking(move || {
            run_inference_loop(&model_dir, cmd_rx, event_tx);
        });

        Ok(Box::new(ParakeetStream { cmd_tx, event_rx }))
    }
}

enum SttCommand {
    Audio(Vec<f32>),
    EndOfUtterance,
    Close,
}

struct ParakeetStream {
    cmd_tx: mpsc::Sender<SttCommand>,
    event_rx: mpsc::Receiver<TranscriptEvent>,
}

impl std::fmt::Debug for ParakeetStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParakeetStream").finish_non_exhaustive()
    }
}

#[async_trait]
impl SttStream for ParakeetStream {
    async fn send_frame(&mut self, pcm: &[i16]) -> Result<()> {
        let samples: Vec<f32> = pcm.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
        self.cmd_tx
            .send(SttCommand::Audio(samples))
            .await
            .map_err(|_| anyhow!("parakeet inference task closed"))
    }

    async fn end_of_utterance(&mut self) -> Result<()> {
        self.cmd_tx
            .send(SttCommand::EndOfUtterance)
            .await
            .map_err(|_| anyhow!("parakeet inference task closed"))
    }

    async fn next_event(&mut self) -> Option<TranscriptEvent> {
        self.event_rx.recv().await
    }

    async fn close(&mut self) -> Result<()> {
        let _ = self.cmd_tx.send(SttCommand::Close).await;
        Ok(())
    }
}

fn run_inference_loop(
    model_dir: &Path,
    mut cmd_rx: mpsc::Receiver<SttCommand>,
    event_tx: mpsc::Sender<TranscriptEvent>,
) {
    use parakeet_rs::Nemotron;

    let mut model = match Nemotron::from_pretrained(model_dir, None) {
        Ok(m) => m,
        Err(e) => {
            let _ = event_tx.blocking_send(TranscriptEvent::Error {
                message: format!("load parakeet model: {e}"),
            });
            return;
        }
    };

    let mut pending: Vec<f32> = Vec::with_capacity(CHUNK_SAMPLES);
    let mut last_partial = String::new();

    while let Some(cmd) = cmd_rx.blocking_recv() {
        match cmd {
            SttCommand::Audio(samples) => {
                pending.extend_from_slice(&samples);
                let mut processed = false;
                while pending.len() >= CHUNK_SAMPLES {
                    let chunk: [f32; CHUNK_SAMPLES] =
                        pending[..CHUNK_SAMPLES].try_into().expect("slice fits");
                    if let Err(e) = model.transcribe_chunk(&chunk) {
                        let _ = event_tx.blocking_send(TranscriptEvent::Error {
                            message: format!("parakeet transcribe_chunk: {e}"),
                        });
                        return;
                    }
                    pending.drain(..CHUNK_SAMPLES);
                    processed = true;
                }
                if processed {
                    let text = model.get_transcript();
                    if !text.is_empty() && text != last_partial {
                        last_partial = text.clone();
                        if event_tx
                            .blocking_send(TranscriptEvent::Partial {
                                text,
                                stability: 0.6,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            }
            SttCommand::EndOfUtterance => {
                if !pending.is_empty() {
                    let mut padded = [0.0f32; CHUNK_SAMPLES];
                    let n = pending.len().min(CHUNK_SAMPLES);
                    padded[..n].copy_from_slice(&pending[..n]);
                    if let Err(e) = model.transcribe_chunk(&padded) {
                        let _ = event_tx.blocking_send(TranscriptEvent::Error {
                            message: format!("parakeet flush: {e}"),
                        });
                        return;
                    }
                    pending.clear();
                }
                let text = model.get_transcript();
                if event_tx
                    .blocking_send(TranscriptEvent::Final {
                        text,
                        words: vec![],
                    })
                    .is_err()
                {
                    return;
                }
                model.reset();
                last_partial.clear();
            }
            SttCommand::Close => return,
        }
    }
}

async fn ensure_model(model_dir: &Path, auto_download: bool) -> Result<()> {
    let missing: Vec<&str> = MODEL_FILES
        .iter()
        .filter(|(name, _)| !model_dir.join(name).is_file())
        .map(|(name, _)| *name)
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    if !auto_download {
        return Err(anyhow!(
            "parakeet model missing files {:?} in {} (auto-download disabled; \
             set CONCH_PARAKEET_MODEL_DIR or run `hf download smcleod/nemotron-speech-streaming-en-0.6b-int8 --local-dir <dir>`)",
            missing,
            model_dir.display()
        ));
    }

    tokio::fs::create_dir_all(model_dir)
        .await
        .with_context(|| format!("create model dir {}", model_dir.display()))?;

    let client = reqwest::Client::builder()
        .user_agent(concat!("conch/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("build http client")?;

    for (name, url) in MODEL_FILES {
        let dest = model_dir.join(name);
        if dest.is_file() {
            continue;
        }
        tracing::info!(file = %name, "downloading parakeet model file");
        let tmp = dest.with_extension("partial");
        let _ = tokio::fs::remove_file(&tmp).await;
        download_to(&client, url, &tmp, name)
            .await
            .with_context(|| format!("downloading {url}"))?;
        tokio::fs::rename(&tmp, &dest)
            .await
            .with_context(|| format!("installing {} -> {}", tmp.display(), dest.display()))?;
    }
    Ok(())
}

async fn download_to(client: &reqwest::Client, url: &str, dest: &Path, name: &str) -> Result<()> {
    download_to_with_progress(client, url, dest, name, |line| eprintln!("{line}")).await
}

async fn download_to_with_progress<F>(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    name: &str,
    mut emit: F,
) -> Result<()>
where
    F: FnMut(String),
{
    use tokio::io::AsyncWriteExt;

    let mut resp = client.get(url).send().await?.error_for_status()?;
    let mut reporter = DownloadProgressReporter::new(name, resp.content_length());
    emit(reporter.start_message());
    let mut file = tokio::fs::File::create(dest).await?;
    let mut downloaded = 0u64;
    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        if let Some(line) = reporter.observe(downloaded) {
            emit(line);
        }
    }
    file.flush().await?;
    if let Some(line) = reporter.finish(downloaded) {
        emit(line);
    }
    Ok(())
}

struct DownloadProgressReporter<'a> {
    name: &'a str,
    total_bytes: Option<u64>,
    last_reported: u64,
}

impl<'a> DownloadProgressReporter<'a> {
    fn new(name: &'a str, total_bytes: Option<u64>) -> Self {
        Self {
            name,
            total_bytes,
            last_reported: 0,
        }
    }

    fn start_message(&self) -> String {
        match self.total_bytes {
            Some(total) if total > 0 => format!(
                "conch: downloading parakeet model file {} ({})...",
                self.name,
                format_bytes(total)
            ),
            _ => format!("conch: downloading parakeet model file {}...", self.name),
        }
    }

    fn observe(&mut self, downloaded: u64) -> Option<String> {
        if !self.should_report(downloaded) {
            return None;
        }
        self.last_reported = downloaded;
        Some(self.progress_message(downloaded))
    }

    fn finish(&mut self, downloaded: u64) -> Option<String> {
        if downloaded == self.last_reported {
            return None;
        }
        self.last_reported = downloaded;
        Some(self.progress_message(downloaded))
    }

    fn should_report(&self, downloaded: u64) -> bool {
        if downloaded <= self.last_reported {
            return false;
        }

        match self.total_bytes {
            Some(total) if total > 0 => {
                let last_pct = self.last_reported.saturating_mul(100) / total;
                let current_pct = downloaded.saturating_mul(100) / total;
                downloaded == total || current_pct >= last_pct + PROGRESS_REPORT_PERCENT_STEP
            }
            _ => downloaded.saturating_sub(self.last_reported) >= PROGRESS_REPORT_BYTES_STEP,
        }
    }

    fn progress_message(&self, downloaded: u64) -> String {
        match self.total_bytes {
            Some(total) if total > 0 => {
                let pct = downloaded
                    .saturating_mul(100)
                    .min(total.saturating_mul(100))
                    / total;
                format!(
                    "conch: parakeet {}: {} / {} ({}%)",
                    self.name,
                    format_bytes(downloaded.min(total)),
                    format_bytes(total),
                    pct
                )
            }
            _ => format!(
                "conch: parakeet {}: {} downloaded",
                self.name,
                format_bytes(downloaded)
            ),
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64;
    let mut unit_idx = 0usize;
    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }
    format!("{value:.1} {}", UNITS[unit_idx])
}

#[derive(Debug)]
struct ScriptedLocalStream {
    finals: std::collections::VecDeque<String>,
}

#[async_trait]
impl SttStream for ScriptedLocalStream {
    async fn send_frame(&mut self, _pcm: &[i16]) -> Result<()> {
        Ok(())
    }

    async fn end_of_utterance(&mut self) -> Result<()> {
        Ok(())
    }

    async fn next_event(&mut self) -> Option<TranscriptEvent> {
        let text = self.finals.pop_front()?;
        Some(TranscriptEvent::Final {
            text,
            words: vec![],
        })
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn progress_reporter_uses_content_length_thresholds() {
        let mut reporter = DownloadProgressReporter::new("encoder.onnx", Some(100));

        assert_eq!(
            reporter.start_message(),
            "conch: downloading parakeet model file encoder.onnx (100 B)..."
        );
        assert_eq!(reporter.observe(4), None);
        assert_eq!(
            reporter.observe(5),
            Some("conch: parakeet encoder.onnx: 5 B / 100 B (5%)".to_string())
        );
        assert_eq!(reporter.observe(9), None);
        assert_eq!(
            reporter.observe(10),
            Some("conch: parakeet encoder.onnx: 10 B / 100 B (10%)".to_string())
        );
        assert_eq!(
            reporter.finish(100),
            Some("conch: parakeet encoder.onnx: 100 B / 100 B (100%)".to_string())
        );
        assert_eq!(reporter.finish(100), None);
    }

    #[test]
    fn progress_reporter_without_content_length_uses_byte_thresholds() {
        let mut reporter = DownloadProgressReporter::new("encoder.onnx", None);
        let step = PROGRESS_REPORT_BYTES_STEP;

        assert_eq!(
            reporter.start_message(),
            "conch: downloading parakeet model file encoder.onnx..."
        );
        assert_eq!(reporter.observe(step - 1), None);
        assert_eq!(
            reporter.observe(step),
            Some(format!(
                "conch: parakeet encoder.onnx: {} downloaded",
                format_bytes(step)
            ))
        );
        assert_eq!(
            reporter.finish(step + 512),
            Some(format!(
                "conch: parakeet encoder.onnx: {} downloaded",
                format_bytes(step + 512)
            ))
        );
    }

    #[tokio::test]
    async fn download_to_with_progress_writes_file_and_emits_messages() {
        let server = MockServer::start().await;
        let body = vec![42u8; 4096];
        Mock::given(method("GET"))
            .and(path("/encoder.onnx"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
            .mount(&server)
            .await;

        let tmp = tempfile::tempdir().expect("tempdir");
        let dest = tmp.path().join("encoder.onnx.partial");
        let client = reqwest::Client::builder().build().expect("client");
        let mut lines = Vec::new();

        download_to_with_progress(
            &client,
            &format!("{}/encoder.onnx", server.uri()),
            &dest,
            "encoder.onnx",
            |line| lines.push(line),
        )
        .await
        .expect("download succeeds");

        assert_eq!(tokio::fs::read(&dest).await.expect("downloaded file"), body);
        assert_eq!(
            lines.first().expect("start line"),
            "conch: downloading parakeet model file encoder.onnx (4.0 KiB)..."
        );
        assert!(
            lines.iter().any(|line| line.contains("(100%)")),
            "expected a final 100% progress line, got {lines:?}"
        );
    }
}
