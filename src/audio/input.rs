use anyhow::Result;
use async_trait::async_trait;
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
        Self { frames: frames.into(), interval }
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
    let supported = device.default_input_config()?;
    let device_rate = supported.sample_rate().0;
    let device_channels = supported.channels();

    let samples_per_frame_device = ((device_rate as u64 * frame_ms as u64) / 1000) as usize
        * device_channels as usize;
    let mut buf: Vec<i16> = Vec::with_capacity(samples_per_frame_device);

    let config: cpal::StreamConfig = supported.into();
    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &_| {
            for &s in data {
                buf.push((s * i16::MAX as f32) as i16);
                if buf.len() >= samples_per_frame_device {
                    // Downmix to mono.
                    let mono: Vec<i16> = if device_channels == 1 {
                        buf.clone()
                    } else {
                        buf.chunks_exact(device_channels as usize)
                            .map(|c| {
                                let sum: i32 = c.iter().map(|&s| s as i32).sum();
                                (sum / device_channels as i32) as i16
                            })
                            .collect()
                    };
                    let resampled = if device_rate == 16_000 {
                        mono
                    } else {
                        match crate::audio::convert::resample_i16(&mono, device_rate, 16_000, 1) {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::warn!("resample failed, dropping frame: {e}");
                                buf.clear();
                                continue;
                            }
                        }
                    };
                    let _ = tx.send(Frame { pcm: resampled });
                    buf.clear();
                }
            }
        },
        |e| tracing::error!("cpal input stream error: {e}"),
        None,
    )?;
    stream.play()?;
    Ok(stream)
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
