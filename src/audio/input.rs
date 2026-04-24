use anyhow::Result;
use async_trait::async_trait;
use cpal::SampleFormat;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub pcm: Vec<i16>,
}

#[async_trait]
pub trait MicSource: Send {
    /// Returns the next frame, or None when the source is exhausted.
    async fn next_frame(&mut self) -> Option<Frame>;
}

pub struct VecMicSource {
    frames: std::collections::VecDeque<Frame>,
    interval: Duration,
}

impl VecMicSource {
    pub fn new(frames: Vec<Frame>, interval: Duration) -> Self {
        Self {
            frames: frames.into(),
            interval,
        }
    }
}

#[async_trait]
impl MicSource for VecMicSource {
    async fn next_frame(&mut self) -> Option<Frame> {
        if self.frames.is_empty() {
            return None;
        }
        sleep(self.interval).await;
        self.frames.pop_front()
    }
}

pub struct MicGate {
    source: Box<dyn MicSource>,
    tx: broadcast::Sender<Frame>,
    open: Arc<AtomicBool>,
}

impl MicGate {
    pub fn new(source: Box<dyn MicSource>, tx: broadcast::Sender<Frame>) -> Self {
        Self {
            source,
            tx,
            open: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Clone a handle that can toggle the gate from outside the run task.
    pub fn toggle_handle(&self) -> MicGateHandle {
        MicGateHandle {
            open: self.open.clone(),
        }
    }

    pub fn set_open(&mut self, open: bool) {
        self.open.store(open, Ordering::SeqCst);
    }

    pub async fn run(mut self) -> Result<()> {
        while let Some(frame) = self.source.next_frame().await {
            if self.open.load(Ordering::SeqCst) {
                // Drop on send error (no subscribers).
                let _ = self.tx.send(frame);
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct MicGateHandle {
    open: Arc<AtomicBool>,
}

impl MicGateHandle {
    pub fn set_open(&self, open: bool) {
        self.open.store(open, Ordering::SeqCst);
    }

    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::SeqCst)
    }
}

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Minimum interval between cpal input error log lines.
///
/// ALSA/PipeWire xruns can fire the stream error callback hundreds of times
/// per second during the first moments of stream startup. Without a rate
/// limit the log flood either corrupts the TUI (stderr paints over frames)
/// or drowns out every other diagnostic.
const INPUT_ERR_LOG_INTERVAL: Duration = Duration::from_millis(500);

/// CPAL-backed mic source at 16 kHz mono s16.
///
/// `cpal::Stream` is not `Send` on all platforms, so the stream is owned by a
/// dedicated OS thread. That thread builds the stream, pushes frames into a
/// channel, and parks until a shutdown signal tears the stream down.
pub struct CpalMicSource {
    rx: tokio::sync::mpsc::UnboundedReceiver<Frame>,
    shutdown: Option<std::sync::mpsc::Sender<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl CpalMicSource {
    pub fn new_default(frame_ms: u32) -> Result<Self> {
        let (frame_tx, frame_rx) = tokio::sync::mpsc::unbounded_channel::<Frame>();
        let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();
        let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<()>>();

        let worker = std::thread::spawn(move || {
            let stream = match build_cpal_input_stream(frame_ms, frame_tx) {
                Ok(s) => {
                    let _ = init_tx.send(Ok(()));
                    s
                }
                Err(e) => {
                    let _ = init_tx.send(Err(e));
                    return;
                }
            };
            // Keep the stream alive until shutdown.
            let _ = shutdown_rx.recv();
            drop(stream);
        });

        // Wait for the worker to confirm stream construction succeeded.
        match init_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                rx: frame_rx,
                shutdown: Some(shutdown_tx),
                worker: Some(worker),
            }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow::anyhow!("cpal worker thread died during init")),
        }
    }
}

fn build_cpal_input_stream(
    frame_ms: u32,
    tx: tokio::sync::mpsc::UnboundedSender<Frame>,
) -> Result<cpal::Stream> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("no default input device"))?;

    // Rate-limit err_callback log spam. Shared across retry attempts so a
    // successfully-opened stream inherits accumulated counters. Stored as
    // millis-since-UNIX_EPOCH so the closure stays Send + 'static.
    let last_err_log_ms = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let err_suppressed = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Build an ordered list of candidate configs to try. Default first
    // (usually best quality + lowest xrun risk); then every supported-range
    // variant with its max sample rate. Some ALSA/PipeWire setups reject
    // the default config's buffer size entirely
    // (snd_pcm_hw_params_set_buffer_size: Invalid argument) — in that case
    // we need to walk supported configs until one opens.
    let mut candidates: Vec<cpal::SupportedStreamConfig> = Vec::new();
    if let Ok(default) = device.default_input_config() {
        candidates.push(default);
    }
    if let Ok(iter) = device.supported_input_configs() {
        for range in iter {
            let cfg = range.with_max_sample_rate();
            if !candidates.iter().any(|c| {
                c.sample_format() == cfg.sample_format()
                    && c.sample_rate() == cfg.sample_rate()
                    && c.channels() == cfg.channels()
            }) {
                candidates.push(cfg);
            }
        }
    }
    if candidates.is_empty() {
        return Err(anyhow::anyhow!("no supported input configs"));
    }

    // For each config, try first with an explicit ~30ms buffer (sweet spot
    // that avoids POLLERR under PipeWire's ALSA-PCM compat plugin and
    // PulseAudio's ALSA bridge), then fall back to BufferSize::Default for
    // ALSA backends that reject fixed sizes outright.
    const TARGET_BUFFER_MS: u32 = 30;
    let mut last_err: Option<anyhow::Error> = None;
    let candidate_attempts: Vec<InputConfigAttempt> = candidates
        .into_iter()
        .map(|supported| {
            let device_rate = supported.sample_rate().0;
            let device_channels = supported.channels();
            let sample_format = supported.sample_format();
            let buffer_range = *supported.buffer_size();
            let config: cpal::StreamConfig = supported.into();
            let fixed_buffer = match buffer_range {
                cpal::SupportedBufferSize::Range { min, max } => {
                    let target =
                        ((device_rate as u64 * TARGET_BUFFER_MS as u64) / 1000).max(1) as u32;
                    Some(cpal::BufferSize::Fixed(target.clamp(min, max)))
                }
                cpal::SupportedBufferSize::Unknown => None,
            };
            InputConfigAttempt {
                config,
                device_rate,
                device_channels,
                sample_format,
                fixed_buffer,
            }
        })
        .collect();

    for prefer_fixed in [true, false] {
        for attempt in &candidate_attempts {
            let Some(buffer_size) = attempt.buffer_size_for_pass(prefer_fixed) else {
                continue;
            };
            let mut cfg = attempt.config.clone();
            cfg.buffer_size = buffer_size;
            match try_build_input_stream(
                &device,
                &cfg,
                attempt.sample_format,
                frame_ms,
                attempt.device_rate,
                attempt.device_channels,
                tx.clone(),
                Arc::clone(&last_err_log_ms),
                Arc::clone(&err_suppressed),
            ) {
                Ok(stream) => {
                    tracing::info!(
                        rate = attempt.device_rate,
                        channels = attempt.device_channels,
                        sample_format = %attempt.sample_format,
                        buffer_size = ?cfg.buffer_size,
                        "cpal input stream opened"
                    );
                    return Ok(stream);
                }
                Err(e) => {
                    tracing::debug!(
                        rate = attempt.device_rate,
                        channels = attempt.device_channels,
                        sample_format = %attempt.sample_format,
                        buffer_size = ?cfg.buffer_size,
                        err = %e,
                        "cpal input config attempt failed"
                    );
                    last_err = Some(e);
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("cpal input stream build failed")))
}

#[derive(Clone)]
struct InputConfigAttempt {
    config: cpal::StreamConfig,
    device_rate: u32,
    device_channels: u16,
    sample_format: SampleFormat,
    fixed_buffer: Option<cpal::BufferSize>,
}

impl InputConfigAttempt {
    fn buffer_size_for_pass(&self, prefer_fixed: bool) -> Option<cpal::BufferSize> {
        match (prefer_fixed, self.fixed_buffer) {
            (true, Some(size)) => Some(size),
            (true, None) => None,
            (false, _) => Some(cpal::BufferSize::Default),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn try_build_input_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: SampleFormat,
    frame_ms: u32,
    device_rate: u32,
    device_channels: u16,
    tx: tokio::sync::mpsc::UnboundedSender<Frame>,
    last_err_log_ms: Arc<std::sync::atomic::AtomicU64>,
    err_suppressed: Arc<std::sync::atomic::AtomicU64>,
) -> Result<cpal::Stream> {
    let stream = match sample_format {
        SampleFormat::F32 => build_input_stream_for_type(
            device,
            config,
            frame_ms,
            device_rate,
            device_channels,
            tx,
            last_err_log_ms,
            err_suppressed,
            |sample: f32| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16,
        )?,
        SampleFormat::I16 => build_input_stream_for_type(
            device,
            config,
            frame_ms,
            device_rate,
            device_channels,
            tx,
            last_err_log_ms,
            err_suppressed,
            |sample: i16| sample,
        )?,
        SampleFormat::U16 => build_input_stream_for_type(
            device,
            config,
            frame_ms,
            device_rate,
            device_channels,
            tx,
            last_err_log_ms,
            err_suppressed,
            |sample: u16| (sample as i32 - i16::MAX as i32 - 1) as i16,
        )?,
        other => {
            return Err(anyhow::anyhow!(
                "unsupported input sample format for CpalMicSource: {other}"
            ));
        }
    };
    stream.play()?;
    Ok(stream)
}

#[allow(clippy::too_many_arguments)]
fn build_input_stream_for_type<T, F>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    frame_ms: u32,
    device_rate: u32,
    device_channels: u16,
    tx: tokio::sync::mpsc::UnboundedSender<Frame>,
    last_err_log_ms: Arc<std::sync::atomic::AtomicU64>,
    err_suppressed: Arc<std::sync::atomic::AtomicU64>,
    mut normalize: F,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample,
    F: FnMut(T) -> i16 + Send + 'static,
{
    let samples_per_frame_device =
        ((device_rate as u64 * frame_ms as u64) / 1000) as usize * device_channels as usize;
    let mut assembler =
        InputFrameAssembler::new(device_rate, device_channels, samples_per_frame_device, tx);

    let stream = device.build_input_stream(
        config,
        move |data: &[T], _: &_| {
            assembler.push_input(data, &mut normalize);
        },
        input_err_fn(last_err_log_ms, err_suppressed),
        None,
    )?;
    Ok(stream)
}

fn input_err_fn(
    last_err_log_ms: Arc<std::sync::atomic::AtomicU64>,
    err_suppressed: Arc<std::sync::atomic::AtomicU64>,
) -> impl FnMut(cpal::StreamError) + Send + 'static {
    move |e| {
        use std::sync::atomic::Ordering::Relaxed;
        use std::time::{SystemTime, UNIX_EPOCH};
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let last = last_err_log_ms.load(Relaxed);
        if now_ms.saturating_sub(last) >= INPUT_ERR_LOG_INTERVAL.as_millis() as u64 {
            let suppressed = err_suppressed.swap(0, Relaxed);
            last_err_log_ms.store(now_ms, Relaxed);
            if suppressed > 0 {
                tracing::error!(
                    suppressed,
                    "cpal input stream error: {e} (+{suppressed} suppressed in last {}ms)",
                    INPUT_ERR_LOG_INTERVAL.as_millis()
                );
            } else {
                tracing::error!("cpal input stream error: {e}");
            }
        } else {
            err_suppressed.fetch_add(1, Relaxed);
        }
    }
}

struct InputFrameAssembler {
    device_rate: u32,
    device_channels: u16,
    samples_per_frame_device: usize,
    tx: tokio::sync::mpsc::UnboundedSender<Frame>,
    buf: Vec<i16>,
}

impl InputFrameAssembler {
    fn new(
        device_rate: u32,
        device_channels: u16,
        samples_per_frame_device: usize,
        tx: tokio::sync::mpsc::UnboundedSender<Frame>,
    ) -> Self {
        Self {
            device_rate,
            device_channels,
            samples_per_frame_device,
            tx,
            buf: Vec::with_capacity(samples_per_frame_device),
        }
    }

    fn push_input<T, F>(&mut self, data: &[T], normalize: &mut F)
    where
        T: Copy,
        F: FnMut(T) -> i16,
    {
        for &sample in data {
            self.buf.push(normalize(sample));
            if self.buf.len() >= self.samples_per_frame_device {
                let mono = self.mix_to_mono();
                let resampled = self.resample_to_stt_rate(mono);
                if let Some(pcm) = resampled {
                    let _ = self.tx.send(Frame { pcm });
                }
                self.buf.clear();
            }
        }
    }

    fn mix_to_mono(&self) -> Vec<i16> {
        if self.device_channels == 1 {
            return self.buf.clone();
        }
        self.buf
            .chunks_exact(self.device_channels as usize)
            .map(|chunk| {
                let sum: i32 = chunk.iter().map(|&sample| sample as i32).sum();
                (sum / self.device_channels as i32) as i16
            })
            .collect()
    }

    fn resample_to_stt_rate(&self, mono: Vec<i16>) -> Option<Vec<i16>> {
        if self.device_rate == 16_000 {
            return Some(mono);
        }
        match crate::audio::convert::resample_i16(&mono, self.device_rate, 16_000, 1) {
            Ok(resampled) => Some(resampled),
            Err(e) => {
                tracing::warn!("resample failed, dropping frame: {e}");
                None
            }
        }
    }
}

impl Drop for CpalMicSource {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

#[async_trait]
impl MicSource for CpalMicSource {
    async fn next_frame(&mut self) -> Option<Frame> {
        self.rx.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_frame_assembler_passes_through_i16_mono_frames() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut assembler = InputFrameAssembler::new(16_000, 1, 4, tx);

        assembler.push_input(&[100i16, -200, 300, -400], &mut |sample| sample);

        let frame = rx.try_recv().expect("frame");
        assert_eq!(frame.pcm, vec![100, -200, 300, -400]);
    }

    #[test]
    fn input_frame_assembler_downmixes_stereo_before_send() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut assembler = InputFrameAssembler::new(16_000, 2, 4, tx);

        assembler.push_input(&[100i16, -100, 200, 0], &mut |sample| sample);

        let frame = rx.try_recv().expect("frame");
        assert_eq!(frame.pcm, vec![0, 100]);
    }

    #[test]
    fn input_frame_assembler_normalizes_u16_to_centered_i16() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut assembler = InputFrameAssembler::new(16_000, 1, 3, tx);

        assembler.push_input(&[0u16, 32768u16, u16::MAX], &mut |sample| {
            (sample as i32 - i16::MAX as i32 - 1) as i16
        });

        let frame = rx.try_recv().expect("frame");
        assert_eq!(frame.pcm.len(), 3);
        assert!(frame.pcm[0] < 0);
        assert_eq!(frame.pcm[1], 0);
        assert!(frame.pcm[2] > 0);
    }
}
