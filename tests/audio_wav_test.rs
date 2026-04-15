use conch::audio::wav::WavSessionWriter;
use hound::WavReader;
use tempfile::TempDir;

#[test]
fn writer_finalizes_on_drop_and_wav_is_readable() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("out.wav");

    {
        let mut w = WavSessionWriter::create(&path, 16_000, 1).unwrap();
        w.write_i16(&[0i16, 1, 2, 3, 4, 5]).unwrap();
    } // drop finalizes

    let reader = WavReader::open(&path).unwrap();
    let spec = reader.spec();
    assert_eq!(spec.sample_rate, 16_000);
    assert_eq!(spec.channels, 1);
    let samples: Vec<i16> = reader.into_samples::<i16>().map(|r| r.unwrap()).collect();
    assert_eq!(samples, vec![0i16, 1, 2, 3, 4, 5]);
}

#[test]
fn writer_survives_panic_mid_stream() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("panic.wav");
    let path_clone = path.clone();

    let handle = std::thread::spawn(move || {
        let mut w = WavSessionWriter::create(&path_clone, 16_000, 1).unwrap();
        w.write_i16(&[10i16, 11, 12]).unwrap();
        panic!("boom");
    });
    let _ = handle.join(); // ignore panic

    let reader = WavReader::open(&path).unwrap();
    let samples: Vec<i16> = reader.into_samples::<i16>().map(|r| r.unwrap()).collect();
    assert_eq!(samples, vec![10i16, 11, 12]);
}
