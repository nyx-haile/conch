use conch::audio::output::{AudioSink, DrainWait, PlaybackTap, RecordingSink, VecSink};
use conch::audio::wav::WavSessionWriter;
use hound::WavReader;
use std::sync::{Arc, Mutex};
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
fn recording_sink_forwards_drain_to_inner_sink() {
    struct DrainSink {
        drained: Arc<Mutex<bool>>,
    }

    impl AudioSink for DrainSink {
        fn push(&mut self, _pcm: Vec<i16>, _sample_rate: u32) -> anyhow::Result<()> {
            Ok(())
        }

        fn drain(&mut self) -> anyhow::Result<DrainWait> {
            *self.drained.lock().unwrap() = true;
            Ok(DrainWait::Complete)
        }

        fn stop(&mut self) {}
    }

    let tmp = TempDir::new().unwrap();
    let drained = Arc::new(Mutex::new(false));
    let sink = DrainSink {
        drained: drained.clone(),
    };
    let mut sink = RecordingSink::new(Box::new(sink), tmp.path().join("tts.wav"));

    sink.drain().unwrap();

    assert!(*drained.lock().unwrap());
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

#[test]
fn recording_sink_writes_session_wav_and_forwards_pcm() {
    let tmp = TempDir::new().unwrap();
    let wav_path = tmp.path().join("session-tts.wav");

    let sink = VecSink::new();
    let sink_handle = sink.collected();
    let mut sink = RecordingSink::new(Box::new(sink), wav_path.clone());

    sink.push(vec![10i16, 20, 30], 22_050).unwrap();
    sink.push(vec![40, 50], 22_050).unwrap();
    drop(sink);

    assert_eq!(
        sink_handle.lock().unwrap().clone(),
        vec![10i16, 20, 30, 40, 50]
    );

    let reader = WavReader::open(&wav_path).unwrap();
    let samples: Vec<i16> = reader.into_samples::<i16>().map(|r| r.unwrap()).collect();
    assert_eq!(samples, vec![10, 20, 30, 40, 50]);
}
