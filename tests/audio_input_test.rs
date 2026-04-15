use conch::audio::input::{Frame, MicGate, VecMicSource};
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn mic_gate_forwards_frames_when_open() {
    let frames = vec![
        Frame { pcm: vec![1i16, 2, 3] },
        Frame { pcm: vec![4, 5, 6] },
    ];
    let source = VecMicSource::new(frames, Duration::from_millis(10));
    let (tx, mut rx) = broadcast::channel(16);
    let mut gate = MicGate::new(Box::new(source), tx);
    gate.set_open(true);

    tokio::spawn(async move { gate.run().await });

    let a = timeout(Duration::from_millis(200), rx.recv()).await.unwrap().unwrap();
    let b = timeout(Duration::from_millis(200), rx.recv()).await.unwrap().unwrap();
    assert_eq!(a.pcm, vec![1, 2, 3]);
    assert_eq!(b.pcm, vec![4, 5, 6]);
}

#[tokio::test]
async fn mic_gate_suppresses_frames_when_closed() {
    let frames = vec![Frame { pcm: vec![1i16] }, Frame { pcm: vec![2] }];
    let source = VecMicSource::new(frames, Duration::from_millis(10));
    let (tx, mut rx) = broadcast::channel(16);
    let mut gate = MicGate::new(Box::new(source), tx);
    gate.set_open(false);
    tokio::spawn(async move { gate.run().await });

    // With the gate closed we should never observe a successful frame.
    // `recv` may either time out (Err from `timeout`) or see the channel close
    // once the gate task exhausts its source (Ok(Err(Closed))). Both are fine;
    // what must NOT happen is a successful `Ok(Ok(frame))`.
    let result = timeout(Duration::from_millis(100), rx.recv()).await;
    match result {
        Err(_) => {}           // timed out: gate correctly swallowed frames
        Ok(Err(_)) => {}       // channel closed after gate finished, still no frame
        Ok(Ok(frame)) => panic!("no frames should be forwarded when gate closed, got {frame:?}"),
    }
}
