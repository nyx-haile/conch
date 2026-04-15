use anyhow::{anyhow, Context, Result};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::io::Cursor;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub fn resample_i16(input: &[i16], from_rate: u32, to_rate: u32, channels: u16) -> Result<Vec<i16>> {
    if from_rate == to_rate {
        return Ok(input.to_vec());
    }
    if channels != 1 {
        return Err(anyhow!("resample_i16 only supports mono input"));
    }
    let ratio = to_rate as f64 / from_rate as f64;
    let params = SincInterpolationParameters {
        sinc_len: 128,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 128,
        window: WindowFunction::BlackmanHarris2,
    };
    let chunk_size = 1024;
    let mut resampler: SincFixedIn<f32> =
        SincFixedIn::new(ratio, 2.0, params, chunk_size, 1).context("creating resampler")?;

    let float_in: Vec<f32> = input.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
    let mut out_samples: Vec<f32> = Vec::new();
    let mut pos = 0;
    while pos + chunk_size <= float_in.len() {
        let slice = &float_in[pos..pos + chunk_size];
        let out = resampler
            .process(&[slice.to_vec()], None)
            .context("resample chunk")?;
        out_samples.extend_from_slice(&out[0]);
        pos += chunk_size;
    }
    if pos < float_in.len() {
        let mut tail = float_in[pos..].to_vec();
        tail.resize(chunk_size, 0.0);
        let out = resampler
            .process(&[tail], None)
            .context("resample tail")?;
        out_samples.extend_from_slice(&out[0]);
    }

    Ok(out_samples
        .iter()
        .map(|&f| (f * i16::MAX as f32) as i16)
        .collect())
}

pub fn decode_mp3_to_pcm(bytes: &[u8]) -> Result<(Vec<i16>, u32)> {
    let cursor = Cursor::new(bytes.to_vec());
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
    let hint = Hint::new();
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .context("probing mp3")?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| anyhow!("no default track"))?;
    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| anyhow!("unknown sample rate"))?;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("creating mp3 decoder")?;

    let mut pcm: Vec<i16> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(e) => return Err(anyhow!("mp3 packet error: {e}")),
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                let mut buf = SampleBuffer::<i16>::new(decoded.capacity() as u64, spec);
                buf.copy_interleaved_ref(decoded);
                // Downmix interleaved stereo to mono by averaging if needed.
                if spec.channels.count() == 1 {
                    pcm.extend_from_slice(buf.samples());
                } else {
                    for frame in buf.samples().chunks_exact(spec.channels.count()) {
                        let sum: i32 = frame.iter().map(|&s| s as i32).sum();
                        pcm.push((sum / spec.channels.count() as i32) as i16);
                    }
                }
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(anyhow!("decode error: {e}")),
        }
    }

    Ok((pcm, sample_rate))
}
