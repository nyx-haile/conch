use anyhow::{Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::fs::{File, OpenOptions};
use std::io::BufWriter;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
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
        let file =
            private_create(path).with_context(|| format!("creating wav at {}", path.display()))?;
        let writer = WavWriter::new(BufWriter::new(file), spec)
            .with_context(|| format!("creating wav at {}", path.display()))?;
        Ok(Self {
            writer: Some(writer),
        })
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

fn private_create(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let file = options
        .open(path)
        .with_context(|| format!("creating {}", path.display()))?;
    #[cfg(unix)]
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("setting private permissions on {}", path.display()))?;
    Ok(file)
}

impl Drop for WavSessionWriter {
    fn drop(&mut self) {
        if let Some(w) = self.writer.take() {
            let _ = w.finalize();
        }
    }
}
