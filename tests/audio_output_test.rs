use conch::audio::output::{AudioSink, PlaybackTap, VecSink};
use conch::audio::wav::WavSessionWriter;
use hound::WavReader;
use tempfile::TempDir;

#[test]
fn playback_tap_fans_out_to_sink_and_wav() {
    let tmp = TempDir::new().unwrap();
    let wav_path = tmp.path().join("tts.wav");

    let sink = VecSink::new();
    let sink_handle = sink.collected();
    let writer = WavSessionWriter::create(&wav_path, 24_000, 1).unwrap();

    let mut tap = PlaybackTap::new(Box::new(sink), writer, 24_000);
    tap.push(vec![1i16, 2, 3]).unwrap();
    tap.push(vec![4, 5]).unwrap();
    drop(tap); // finalizes wav

    assert_eq!(sink_handle.lock().unwrap().clone(), vec![1i16, 2, 3, 4, 5]);

    let reader = WavReader::open(&wav_path).unwrap();
    let samples: Vec<i16> = reader.into_samples::<i16>().map(|r| r.unwrap()).collect();
    assert_eq!(samples, vec![1, 2, 3, 4, 5]);
}

#[test]
fn vec_sink_supports_clear_and_stop() {
    let sink = VecSink::new();
    let handle = sink.collected();
    let mut sink = sink;
    sink.push(vec![1, 2, 3], 24_000).unwrap();
    sink.stop();
    sink.push(vec![4, 5], 24_000).unwrap();
    assert_eq!(*handle.lock().unwrap(), vec![1, 2, 3]);
}
