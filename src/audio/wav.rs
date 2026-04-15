use anyhow::{Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub struct WavSessionWriter {
    writer: Option<WavWriter<BufWriter<File>>>,
}

impl WavSessionWriter {
    pub fn create(path: &Path, sample_rate: u32, channels: u16) -> Result<Self> {
        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let writer = WavWriter::create(path, spec)
            .with_context(|| format!("creating wav at {}", path.display()))?;
        Ok(Self { writer: Some(writer) })
    }

    pub fn write_i16(&mut self, samples: &[i16]) -> Result<()> {
        let w = self.writer.as_mut().context("writer already finalized")?;
        for s in samples {
            w.write_sample(*s).context("writing sample")?;
        }
        w.flush().context("flushing wav")?;
        Ok(())
    }
}

impl Drop for WavSessionWriter {
    fn drop(&mut self) {
        if let Some(w) = self.writer.take() {
            let _ = w.finalize();
        }
    }
}
