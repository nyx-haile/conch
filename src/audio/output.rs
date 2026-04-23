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
// RecordingSink — forwards PCM to a live sink and lazily records a session wav
// ---------------------------------------------------------------------------

pub struct RecordingSink {
    inner: Box<dyn AudioSink>,
    wav_path: std::path::PathBuf,
    writer: Option<WavSessionWriter>,
    sample_rate: Option<u32>,
}

impl RecordingSink {
    pub fn new(inner: Box<dyn AudioSink>, wav_path: std::path::PathBuf) -> Self {
        Self {
            inner,
            wav_path,
            writer: None,
            sample_rate: None,
        }
    }

    fn ensure_writer(&mut self, sample_rate: u32) {
        if self.writer.is_some() {
            return;
        }
        match WavSessionWriter::create(&self.wav_path, sample_rate, 1) {
            Ok(writer) => {
                self.writer = Some(writer);
                self.sample_rate = Some(sample_rate);
            }
            Err(e) => {
                tracing::warn!(
                    err = %e,
                    path = %self.wav_path.display(),
                    "failed to create session TTS wav"
                );
            }
        }
    }
}

impl AudioSink for RecordingSink {
    fn push(&mut self, pcm: Vec<i16>, sample_rate: u32) -> Result<()> {
        self.ensure_writer(sample_rate);
        if let Some(writer) = self.writer.as_mut() {
            if self.sample_rate == Some(sample_rate) {
                writer.write_i16(&pcm)?;
            } else {
                tracing::warn!(
                    initial_rate = ?self.sample_rate,
                    current_rate = sample_rate,
                    "session TTS wav sample rate changed; skipping wav append"
                );
            }
        }
        self.inner.push(pcm, sample_rate)
    }

    fn stop(&mut self) {
        self.inner.stop();
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
use rodio::{DeviceSinkBuilder, Player};
use std::num::NonZero;

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
            // `open_default_sink()` tries the default device's default config,
            // then falls back through every supported config and alternate
            // device until one opens. On ALSA setups where the default config
            // is unacceptable (e.g. `snd_pcm_hw_params_set_buffer_size`
            // invalid arg) this is what lets playback succeed. Rodio's own
            // errors flow through its tracing feature into our file log.
            let mut sink_device = match DeviceSinkBuilder::open_default_sink() {
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
