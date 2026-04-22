use conch::audio::convert::{decode_mp3_to_pcm, resample_i16};

#[test]
fn resample_halves_length_when_rate_halved() {
    let input: Vec<i16> = (0..4800).map(|i| (i as i16) % 100).collect();
    let out = resample_i16(&input, 48_000, 16_000, 1).unwrap();
    // 48000→16000 = /3; allow rubato's filter tail tolerance (+/- 64 samples)
    let expected = input.len() / 3;
    assert!(
        (out.len() as isize - expected as isize).abs() < 128,
        "expected ~{} samples, got {}",
        expected,
        out.len()
    );
}

#[test]
fn resample_is_identity_when_rates_match() {
    let input: Vec<i16> = vec![1, 2, 3, 4, 5];
    let out = resample_i16(&input, 16_000, 16_000, 1).unwrap();
    assert_eq!(out, input);
}

#[test]
fn resample_does_not_leak_tail_padding() {
    // 4800 samples is not a multiple of chunk_size=1024, so the final
    // partial chunk is zero-padded internally. Regression: that padding
    // used to leak into the output and inflate its length (pre-fix the
    // output was ~1684, +84 over the expected 1600).
    //
    // SincFixedIn has an inherent ~22-sample priming delay on the first
    // chunk, so the tolerance is ±32 — still well under the ±128 "filter
    // tail" slack of the companion test and tight enough to catch the
    // old padding-leak (+84) regression.
    let input: Vec<i16> = (0..4800)
        .map(|i| ((i as f32 * 0.1).sin() * 10_000.0) as i16)
        .collect();
    let out = resample_i16(&input, 48_000, 16_000, 1).unwrap();
    let expected: isize = 1600;
    let delta = (out.len() as isize - expected).abs();
    assert!(
        delta <= 32,
        "expected {} ± 32 samples, got {} (delta {})",
        expected,
        out.len(),
        delta
    );
}

#[test]
fn decode_mp3_returns_pcm_and_sample_rate() {
    let bytes = std::fs::read("tests/fixtures/tone_200ms_44100.mp3").unwrap();
    let (pcm, rate) = decode_mp3_to_pcm(&bytes).unwrap();
    assert_eq!(rate, 44_100);
    // ~200ms of audio at 44.1kHz = ~8820 samples (mono or stereo-downmixed)
    assert!(pcm.len() > 8000 && pcm.len() < 20_000);
}
