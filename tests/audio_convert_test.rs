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
fn decode_mp3_returns_pcm_and_sample_rate() {
    let bytes = std::fs::read("tests/fixtures/tone_200ms_44100.mp3").unwrap();
    let (pcm, rate) = decode_mp3_to_pcm(&bytes).unwrap();
    assert_eq!(rate, 44_100);
    // ~200ms of audio at 44.1kHz = ~8820 samples (mono or stereo-downmixed)
    assert!(pcm.len() > 8000 && pcm.len() < 20_000);
}
