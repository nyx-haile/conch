use async_trait::async_trait;
use conch::interview::fillers::{
    FillerCache, FillerCategory, DEFAULT_INTERRUPT_FILLERS, DEFAULT_THINKING_FILLERS,
};
use conch::tts::{TextToSpeech, TtsConfig, TtsStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

struct CountingTts {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl TextToSpeech for CountingTts {
    async fn open_stream(&self, _c: &TtsConfig) -> anyhow::Result<Box<dyn TtsStream>> {
        Err(anyhow::anyhow!("stream not used in fillers test"))
    }
    async fn synthesize_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<i16>>> {
        self.calls.fetch_add(texts.len(), Ordering::SeqCst);
        Ok(texts.iter().map(|t| vec![t.len() as i16]).collect())
    }
}

#[tokio::test]
async fn default_libraries_are_nonempty() {
    assert!(DEFAULT_INTERRUPT_FILLERS.len() >= 6);
    assert!(DEFAULT_THINKING_FILLERS.len() >= 8);
}

#[tokio::test]
async fn cache_synthesizes_once_and_is_idempotent_on_second_load() {
    let tmp = TempDir::new().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let tts = CountingTts {
        calls: calls.clone(),
    };

    let mut cache = FillerCache::load_or_new(tmp.path(), "elevenlabs", "voice-a").unwrap();
    cache.sync(&tts).await.unwrap();
    let first = calls.load(Ordering::SeqCst);
    assert_eq!(
        first,
        DEFAULT_INTERRUPT_FILLERS.len() + DEFAULT_THINKING_FILLERS.len()
    );

    let mut cache2 = FillerCache::load_or_new(tmp.path(), "elevenlabs", "voice-a").unwrap();
    cache2.sync(&tts).await.unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        first,
        "second sync should not re-synthesize"
    );

    let pcm = cache2.pick_random(FillerCategory::Interrupt).unwrap();
    assert!(!pcm.is_empty());
}
