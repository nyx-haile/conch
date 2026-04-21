use crate::audio::wav::WavSessionWriter;
use anyhow::Result;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// AudioSink trait
// ---------------------------------------------------------------------------

pub trait AudioSink: Send {
    fn push(&mut self, pcm: Vec<i16>, sample_rate: u32) -> Result<()>;
    fn stop(&mut self);
}

// ---------------------------------------------------------------------------
// VecSink — collects PCM in memory (tests / headless)
// ---------------------------------------------------------------------------

pub struct VecSink {
    collected: Arc<Mutex<Vec<i16>>>,
    stopped: bool,
}

impl VecSink {
    pub fn new() -> Self {
        Self {
            collected: Arc::new(Mutex::new(Vec::new())),
            stopped: false,
        }
    }

    pub fn collected(&self) -> Arc<Mutex<Vec<i16>>> {
        self.collected.clone()
    }
}

impl Default for VecSink {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioSink for VecSink {
    fn push(&mut self, pcm: Vec<i16>, _sample_rate: u32) -> Result<()> {
        if self.stopped {
            return Ok(());
        }
        self.collected.lock().unwrap().extend(pcm);
        Ok(())
    }

    fn stop(&mut self) {
        self.stopped = true;
    }
}

// ---------------------------------------------------------------------------
// PlaybackTap — fans out PCM to an AudioSink and a WAV writer simultaneously
// ---------------------------------------------------------------------------

pub struct PlaybackTap {
    sink: Box<dyn AudioSink>,
    writer: WavSessionWriter,
    sample_rate: u32,
}

impl PlaybackTap {
    pub fn new(sink: Box<dyn AudioSink>, writer: WavSessionWriter, sample_rate: u32) -> Self {
        Self {
            sink,
            writer,
            sample_rate,
        }
    }

    pub fn push(&mut self, pcm: Vec<i16>) -> Result<()> {
        self.writer.write_i16(&pcm)?;
        self.sink.push(pcm, self.sample_rate)
    }

    pub fn stop(&mut self) {
        self.sink.stop();
    }
}

// ---------------------------------------------------------------------------
// RodioSink — real audio output via rodio
//
// `rodio::OutputStream` wraps `cpal::Stream` which is `!Send` on some
// platforms. Since `AudioSink: Send`, we use the same worker-thread pattern
// as `CpalMicSource`: the `OutputStream` + `Sink` live on a dedicated OS
// thread, and we send commands to it over an `mpsc` channel.
// ---------------------------------------------------------------------------

use rodio::buffer::SamplesBuffer;
use rodio::cpal;
use rodio::{DeviceSinkBuilder, Player};
use std::num::NonZero;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

enum RodioCmd {
    Push { pcm: Vec<i16>, sample_rate: u32 },
    Stop,
    Shutdown,
}

pub struct RodioSink {
    cmd_tx: std::sync::mpsc::Sender<RodioCmd>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl RodioSink {
    pub fn new_default() -> Result<Self> {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<RodioCmd>();
        let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<()>>();

        let worker = std::thread::spawn(move || {
            let start = Instant::now();
            let last_err_ms = std::sync::Arc::new(AtomicU64::new(0));
            let error_cb = move |err: cpal::StreamError| {
                let now_ms = start.elapsed().as_millis() as u64;
                let prev = last_err_ms.load(Ordering::Relaxed);
                if now_ms.saturating_sub(prev) >= 5_000
                    && last_err_ms
                        .compare_exchange(prev, now_ms, Ordering::Relaxed, Ordering::Relaxed)
                        .is_ok()
                {
                    tracing::warn!(target: "conch::audio", "rodio output stream error: {err}");
                }
            };

            let builder = match DeviceSinkBuilder::from_default_device() {
                Ok(b) => b.with_error_callback(error_cb),
                Err(e) => {
                    let _ = init_tx.send(Err(anyhow::anyhow!("rodio output device: {e}")));
                    return;
                }
            };

            let mut sink_device = match builder.open_stream() {
                Ok(s) => s,
                Err(e) => {
                    let _ = init_tx.send(Err(anyhow::anyhow!("rodio output stream: {e}")));
                    return;
                }
            };
            sink_device.log_on_drop(false);

            let player = Player::connect_new(sink_device.mixer());

            let _ = init_tx.send(Ok(()));

            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    RodioCmd::Push { pcm, sample_rate } => {
                        let Some(sr_nz) = NonZero::new(sample_rate) else {
                            tracing::warn!(target: "conch::audio", "rodio push ignored: sample_rate=0");
                            continue;
                        };
                        let f32_pcm: Vec<f32> = pcm
                            .into_iter()
                            .map(|s| s as f32 / i16::MAX as f32)
                            .collect();
                        let source = SamplesBuffer::new(NonZero::new(1).unwrap(), sr_nz, f32_pcm);
                        player.append(source);
                    }
                    RodioCmd::Stop => {
                        player.stop();
                        player.clear();
                    }
                    RodioCmd::Shutdown => break,
                }
            }

            drop(player);
            drop(sink_device);
        });

        match init_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                cmd_tx,
                worker: Some(worker),
            }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow::anyhow!("rodio worker thread died during init")),
        }
    }
}

impl Drop for RodioSink {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(RodioCmd::Shutdown);
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

impl AudioSink for RodioSink {
    fn push(&mut self, pcm: Vec<i16>, sample_rate: u32) -> Result<()> {
        self.cmd_tx
            .send(RodioCmd::Push { pcm, sample_rate })
            .map_err(|_| anyhow::anyhow!("rodio worker thread gone"))?;
        Ok(())
    }

    fn stop(&mut self) {
        let _ = self.cmd_tx.send(RodioCmd::Stop);
    }
}
