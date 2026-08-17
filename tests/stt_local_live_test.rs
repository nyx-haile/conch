//! Opt-in live integration test for the local STT backend.
//!
//! `tests/stt_local_test.rs` only covers the scripted-finals shim and the
//! missing-model error path. The two genuinely uncovered paths in
//! `src/stt/local.rs` are:
//!
//!   1. `ensure_model` / `download_to` fetching the pinned HuggingFace bundle
//!      `smcleod/nemotron-speech-streaming-en-0.6b-int8` over the network.
//!   2. `run_inference_loop` actually driving Nemotron over real audio and
//!      emitting a `TranscriptEvent::Final`.
//!
//! Both are exercised here. Everything is gated behind `CONCH_STT_LIVE=1` so
//! the default `cargo test` stays fast, offline, and downloads nothing —
//! without the variable every test in this file returns immediately.
//!
//! ```bash
//! CONCH_STT_LIVE=1 cargo test --test stt_local_live_test -- --nocapture --test-threads=1
//! ```
//!
//! Knobs:
//!   CONCH_STT_LIVE=1                 required; nothing runs without it
//!   CONCH_STT_LIVE_MODEL_DIR=<dir>   bundle to use for the inference test
//!                                    (default: ~/.conch/models/...)
//!   CONCH_STT_LIVE_FULL_DOWNLOAD=1   force the download test to pull all
//!                                    three files (~890 MB) instead of
//!                                    seeding the two large ONNX files from an
//!                                    existing cache and fetching only
//!                                    tokenizer.model over the wire
//!
//! ## Fixture
//!
//! `tests/fixtures/hello-conch-16k.wav` is 3.1 s of synthesized speech saying
//! "Hello conch. This is a test of local speech to text." It was produced with
//! piper + ffmpeg, with piper's noise scales pinned to 0 so the render is
//! deterministic:
//!
//! ```bash
//! printf 'Hello conch. This is a test of local speech to text.' > /tmp/f.txt
//! piper-tts -m en_US-lessac-medium.onnx -c en_US-lessac-medium.onnx.json \
//!   -i /tmp/f.txt -f /tmp/piper-raw.wav \
//!   --noise-scale 0.0 --noise-w-scale 0.0 --sentence-silence 0.1
//! ffmpeg -y -i /tmp/piper-raw.wav -ac 1 -ar 16000 -sample_fmt s16 \
//!   -c:a pcm_s16le tests/fixtures/hello-conch-16k.wav
//! ```
//!
//! It is TTS output, not a human recording. The pinned bundle does transcribe
//! it verbatim today ("Hello, Conch, this is a test of local speech to text"),
//! but the assertions stay on content words rather than an exact string so
//! punctuation/casing drift in a future model revision is not a false alarm.
//!
//! ## Not covered here
//!
//! The ticket also asked for a nightly CI lane. This repository has no CI
//! configuration of any kind (no `.github/`, no other pipeline file), so there
//! is nothing to wire this into; that half of the ticket is deliberately left
//! undone rather than faked.

use conch::stt::{local::LocalStt, SpeechToText, SttConfig, TranscriptEvent};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const LIVE_ENV: &str = "CONCH_STT_LIVE";
const MODEL_FILE_NAMES: [&str; 3] = ["encoder.onnx", "decoder_joint.onnx", "tokenizer.model"];
const FIXTURE: &str = "tests/fixtures/hello-conch-16k.wav";
/// 20 ms at 16 kHz — the frame size the mic path uses.
const FRAME_SAMPLES: usize = 320;

/// True only when the operator explicitly opted in. Everything in this file
/// is a no-op otherwise.
fn live_enabled() -> bool {
    std::env::var(LIVE_ENV).ok().as_deref() == Some("1")
}

fn skip(test: &str) {
    eprintln!("{test}: skipped ({LIVE_ENV} != 1) — live download/inference not exercised");
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Bundle used by the inference test. Never a tempdir: re-downloading ~890 MB
/// per run would make the opt-in lane unusable.
fn cached_model_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CONCH_STT_LIVE_MODEL_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(dir) = std::env::var("CONCH_PARAKEET_MODEL_DIR") {
        return PathBuf::from(dir);
    }
    let home = directories::UserDirs::new()
        .expect("home directory")
        .home_dir()
        .to_path_buf();
    home.join(".conch")
        .join("models")
        .join("parakeet-nemotron-streaming-en-0.6b")
}

/// Link a cached model file into `dest` so `ensure_model` treats it as
/// present. Returns false when it could not be linked (non-unix, or a cache
/// that turned out not to be readable), in which case it gets downloaded.
fn seed_from_cache(cache: &Path, dest: &Path, name: &str) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(cache.join(name), dest.join(name)).is_ok()
    }
    #[cfg(not(unix))]
    {
        let _ = (cache, dest, name);
        false
    }
}

fn bundle_is_complete(dir: &Path) -> bool {
    MODEL_FILE_NAMES.iter().all(|n| dir.join(n).is_file())
}

/// Guard against a stray `CONCH_TEST_SCRIPTED_STT_FINALS` in the operator's
/// shell silently turning the live test into the scripted shim, which would
/// make it pass while exercising nothing.
fn assert_not_scripted() {
    assert!(
        std::env::var_os("CONCH_TEST_SCRIPTED_STT_FINALS").is_none(),
        "CONCH_TEST_SCRIPTED_STT_FINALS is set; unset it — it short-circuits \
         the real download and inference paths this test exists to cover"
    );
    assert!(
        std::env::var_os("CONCH_PARAKEET_NO_DOWNLOAD").is_none(),
        "CONCH_PARAKEET_NO_DOWNLOAD is set; unset it for the live lane"
    );
}

/// Exercises `ensure_model` + `download_to` against the real pinned HF repo.
///
/// By default the two large ONNX files are symlinked in from an existing local
/// bundle so only `tokenizer.model` (~250 KB) crosses the network; that still
/// runs the full reqwest → `.partial` → rename path and would fail if the repo
/// moved or HF changed its download headers. With no local bundle, or with
/// `CONCH_STT_LIVE_FULL_DOWNLOAD=1`, all three files are fetched for real.
#[tokio::test]
async fn live_download_fetches_pinned_bundle_from_huggingface() {
    if !live_enabled() {
        skip("live_download_fetches_pinned_bundle_from_huggingface");
        return;
    }
    assert_not_scripted();

    let tmp = tempfile::tempdir().expect("tempdir");
    let dest = tmp.path().join("bundle");
    std::fs::create_dir_all(&dest).expect("create bundle dir");

    let cache = cached_model_dir();
    let full = std::env::var("CONCH_STT_LIVE_FULL_DOWNLOAD")
        .ok()
        .as_deref()
        == Some("1");
    let mut seeded: Vec<&str> = Vec::new();
    if !full && bundle_is_complete(&cache) {
        // Seed only the big files; tokenizer.model is always fetched live.
        for name in ["encoder.onnx", "decoder_joint.onnx"] {
            if seed_from_cache(&cache, &dest, name) {
                seeded.push(name);
            }
        }
    }
    let expected_downloads: Vec<&str> = MODEL_FILE_NAMES
        .iter()
        .copied()
        .filter(|n| !seeded.contains(n))
        .collect();
    eprintln!(
        "live download: seeding {seeded:?} from {}, fetching {expected_downloads:?} over the network",
        cache.display()
    );
    assert!(
        !expected_downloads.is_empty(),
        "at least one file must be fetched live for this test to mean anything"
    );

    let started = Instant::now();
    LocalStt::new(dest.clone())
        .prepare()
        .await
        .expect("live download from huggingface");
    eprintln!("live download: finished in {:?}", started.elapsed());

    for name in MODEL_FILE_NAMES {
        let path = dest.join(name);
        assert!(path.is_file(), "{name} missing after prepare()");
        let len = std::fs::metadata(&path).expect("metadata").len();
        assert!(
            len > 100_000,
            "{name} is {len} bytes — HF probably served an error page or an \
             LFS pointer instead of the real file"
        );
    }
    // Real ONNX files are protobuf: field 1 (ir_version), varint -> 0x08.
    for name in &expected_downloads {
        if name.ends_with(".onnx") {
            let head = std::fs::read(dest.join(name)).expect("read onnx");
            assert_eq!(head[0], 0x08, "{name} does not look like an ONNX protobuf");
        }
    }
    // download_to writes `<name>.partial` then renames; nothing may be left.
    let leftovers: Vec<String> = std::fs::read_dir(&dest)
        .expect("read bundle dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".partial"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "leftover partial downloads: {leftovers:?}"
    );

    // ensure_model is idempotent: a second prepare() must not re-fetch.
    let before: Vec<u64> = MODEL_FILE_NAMES
        .iter()
        .map(|n| std::fs::metadata(dest.join(n)).expect("metadata").len())
        .collect();
    LocalStt::new(dest.clone())
        .prepare()
        .await
        .expect("second prepare is a no-op");
    let after: Vec<u64> = MODEL_FILE_NAMES
        .iter()
        .map(|n| std::fs::metadata(dest.join(n)).expect("metadata").len())
        .collect();
    assert_eq!(before, after, "prepare() re-downloaded an existing bundle");
}

/// Exercises `run_inference_loop`: real Nemotron over real 16 kHz audio,
/// through `send_frame` → `end_of_utterance` → `TranscriptEvent::Final`.
#[tokio::test]
async fn live_inference_transcribes_wav_fixture_to_a_final_event() {
    if !live_enabled() {
        skip("live_inference_transcribes_wav_fixture_to_a_final_event");
        return;
    }
    assert_not_scripted();

    let fixture = repo_root().join(FIXTURE);
    assert!(fixture.is_file(), "missing fixture {}", fixture.display());
    let samples = read_fixture_pcm(&fixture);
    assert!(
        samples.len() > 16_000,
        "fixture is only {} samples — too short to transcribe",
        samples.len()
    );

    let model_dir = cached_model_dir();
    eprintln!("live inference: model dir {}", model_dir.display());
    let stt = LocalStt::new(model_dir);
    stt.prepare().await.expect("model bundle available");

    let mut stream = stt
        .open_stream(&SttConfig {
            sample_rate: 16_000,
            language: None,
            punctuate: true,
        })
        .await
        .expect("open parakeet stream");

    for frame in samples.chunks(FRAME_SAMPLES) {
        stream.send_frame(frame).await.expect("send frame");
    }
    stream.end_of_utterance().await.expect("end of utterance");

    // Drain until the Final arrives; partials may precede it.
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut partials = 0usize;
    let mut final_text: Option<String> = None;
    while Instant::now() < deadline {
        let remaining = deadline - Instant::now();
        match tokio::time::timeout(remaining, stream.next_event()).await {
            Ok(Some(TranscriptEvent::Partial { text, .. })) => {
                partials += 1;
                eprintln!("partial: {text}");
            }
            Ok(Some(TranscriptEvent::Final { text, .. })) => {
                eprintln!("final: {text}");
                final_text = Some(text);
                break;
            }
            Ok(Some(TranscriptEvent::Error { message })) => {
                panic!("parakeet inference error: {message}");
            }
            Ok(None) => panic!("stream closed before emitting a Final"),
            Err(_) => panic!("timed out waiting for a Final transcript"),
        }
    }
    let _ = stream.close().await;

    let text = final_text.expect("a Final transcript event");
    let normalized: String = text
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    assert!(
        !normalized.trim().is_empty(),
        "Final transcript was empty; raw = {text:?}"
    );
    eprintln!("live inference: {partials} partial(s), final = {text:?}");

    // The fixture is synthetic TTS, so assert on content words rather than an
    // exact string — punctuation and casing are the model's business. On the
    // pinned bundle this currently comes back verbatim as
    // "Hello, Conch, this is a test of local speech to text".
    for word in ["hello", "conch", "test", "local", "speech", "text"] {
        assert!(
            normalized.split_whitespace().any(|w| w == word),
            "expected {word:?} in transcript {text:?}"
        );
    }
}

/// Read the fixture as mono 16 kHz `i16`, asserting the format the STT path
/// expects rather than silently resampling.
fn read_fixture_pcm(path: &Path) -> Vec<i16> {
    let reader = hound::WavReader::open(path).expect("open fixture wav");
    let spec = reader.spec();
    assert_eq!(spec.channels, 1, "fixture must be mono");
    assert_eq!(spec.sample_rate, 16_000, "fixture must be 16 kHz");
    assert_eq!(spec.bits_per_sample, 16, "fixture must be 16-bit");
    reader
        .into_samples::<i16>()
        .collect::<Result<Vec<i16>, _>>()
        .expect("decode fixture samples")
}
