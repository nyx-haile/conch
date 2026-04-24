use crate::audio::wav::WavSessionWriter;
use anyhow::{anyhow, Context, Result};
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
use rodio::cpal;
use rodio::cpal::traits::{DeviceTrait, HostTrait};
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
            let preferred = std::env::var("CONCH_OUTPUT_DEVICE").ok();
            let (device, meta, reason) = match select_output_device(preferred.as_deref()) {
                Ok(s) => s,
                Err(e) => {
                    let _ = init_tx.send(Err(anyhow!("rodio output device: {e}")));
                    return;
                }
            };
            let builder = match DeviceSinkBuilder::from_device(device) {
                Ok(builder) => builder,
                Err(e) => {
                    let _ = init_tx.send(Err(anyhow!("rodio output stream: {e}")));
                    return;
                }
            };
            let mut sink_device = match builder
                .with_error_callback(|err| {
                    tracing::error!(target: "conch::audio", "rodio output stream error: {err}");
                })
                .open_sink_or_fallback()
            {
                Ok(s) => s,
                Err(e) => {
                    let _ = init_tx.send(Err(anyhow!("rodio output stream: {e}")));
                    return;
                }
            };
            sink_device.log_on_drop(false);

            let cfg = *sink_device.config();
            tracing::info!(
                target: "conch::audio",
                device = %meta.name,
                driver = ?meta.driver,
                reason = %reason,
                channels = cfg.channel_count().get(),
                sample_rate = cfg.sample_rate().get(),
                buffer_size = ?cfg.buffer_size(),
                sample_format = %cfg.sample_format(),
                "rodio output sink opened"
            );

            let player = Player::connect_new(sink_device.mixer());
            player.play();

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

#[derive(Clone, Debug, PartialEq, Eq)]
struct OutputDeviceMeta {
    name: String,
    driver: Option<String>,
    is_default: bool,
}

impl OutputDeviceMeta {
    fn matches_query(&self, query: &str) -> bool {
        let q = query.to_ascii_lowercase();
        self.name.to_ascii_lowercase().contains(&q)
            || self
                .driver
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .contains(&q)
    }

    fn is_null(&self) -> bool {
        self.driver.as_deref() == Some("null")
            || self
                .name
                .to_ascii_lowercase()
                .contains("discard all samples")
    }

    fn is_generic_default(&self) -> bool {
        self.driver.as_deref() == Some("default") || self.name == "Default Audio Device"
    }

    fn is_server_device(&self) -> bool {
        matches!(self.driver.as_deref(), Some("pulse") | Some("pipewire"))
    }
}

fn choose_output_device_index(
    devices: &[OutputDeviceMeta],
    preferred: Option<&str>,
) -> anyhow::Result<(usize, &'static str)> {
    if let Some(query) = preferred.filter(|q| !q.trim().is_empty()) {
        if let Some((idx, _)) = devices
            .iter()
            .enumerate()
            .find(|(_, meta)| !meta.is_null() && meta.matches_query(query))
        {
            return Ok((idx, "preferred-env"));
        }
        return Err(anyhow!(
            "no output device matched CONCH_OUTPUT_DEVICE={query:?}"
        ));
    }

    let default_idx = devices
        .iter()
        .position(|meta| meta.is_default && !meta.is_null());
    if let Some(idx) = default_idx {
        if !devices[idx].is_generic_default() {
            return Ok((idx, "system-default"));
        }
    }

    if let Some((idx, _)) = devices
        .iter()
        .enumerate()
        .find(|(_, meta)| !meta.is_null() && meta.is_server_device())
    {
        return Ok((idx, "server-device"));
    }

    if let Some(idx) = default_idx {
        return Ok((idx, "generic-default"));
    }

    devices
        .iter()
        .enumerate()
        .find(|(_, meta)| !meta.is_null())
        .map(|(idx, _)| (idx, "first-non-null"))
        .ok_or_else(|| anyhow!("no usable output device available"))
}

fn select_output_device(
    preferred: Option<&str>,
) -> anyhow::Result<(cpal::Device, OutputDeviceMeta, &'static str)> {
    let host = cpal::default_host();
    let default_key = host
        .default_output_device()
        .and_then(|device| device.description().ok())
        .map(|desc| {
            (
                desc.name().to_string(),
                desc.driver().map(|d| d.to_string()),
            )
        });

    let mut devices = Vec::new();
    for device in host.output_devices().context("listing output devices")? {
        let desc = device.description().context("describing output device")?;
        let key = (
            desc.name().to_string(),
            desc.driver().map(|d| d.to_string()),
        );
        let meta = OutputDeviceMeta {
            name: desc.name().to_string(),
            driver: desc.driver().map(|d| d.to_string()),
            is_default: default_key.as_ref() == Some(&key),
        };
        devices.push((device, meta));
    }

    let metas: Vec<OutputDeviceMeta> = devices.iter().map(|(_, meta)| meta.clone()).collect();
    let (idx, reason) = choose_output_device_index(&metas, preferred)?;
    let (device, meta) = devices
        .into_iter()
        .nth(idx)
        .ok_or_else(|| anyhow!("selected output device index out of bounds"))?;
    Ok((device, meta, reason))
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

#[cfg(test)]
mod tests {
    use super::{choose_output_device_index, OutputDeviceMeta};

    #[test]
    fn output_device_prefers_explicit_env_match() {
        let devices = vec![
            OutputDeviceMeta {
                name: "Default Audio Device".to_string(),
                driver: Some("default".to_string()),
                is_default: true,
            },
            OutputDeviceMeta {
                name: "PulseAudio Sound Server".to_string(),
                driver: Some("pulse".to_string()),
                is_default: false,
            },
        ];
        let (idx, reason) = choose_output_device_index(&devices, Some("pulse")).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(reason, "preferred-env");
    }

    #[test]
    fn output_device_prefers_server_device_over_generic_default() {
        let devices = vec![
            OutputDeviceMeta {
                name: "Default Audio Device".to_string(),
                driver: Some("default".to_string()),
                is_default: true,
            },
            OutputDeviceMeta {
                name: "PulseAudio Sound Server".to_string(),
                driver: Some("pulse".to_string()),
                is_default: false,
            },
        ];
        let (idx, reason) = choose_output_device_index(&devices, None).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(reason, "server-device");
    }

    #[test]
    fn output_device_skips_null_driver() {
        let devices = vec![
            OutputDeviceMeta {
                name: "Discard all samples (playback) or generate zero samples (capture)"
                    .to_string(),
                driver: Some("null".to_string()),
                is_default: true,
            },
            OutputDeviceMeta {
                name: "Built-in Audio".to_string(),
                driver: Some("hw:CARD=0,DEV=0".to_string()),
                is_default: false,
            },
        ];
        let (idx, reason) = choose_output_device_index(&devices, None).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(reason, "first-non-null");
    }
}
