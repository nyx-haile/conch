use crate::stt::{SpeechToText, SttConfig, SttStream, TranscriptEvent};
use anyhow::{anyhow, Result};
use async_trait::async_trait;

pub struct LocalStt;

impl LocalStt {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalStt {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SpeechToText for LocalStt {
    async fn open_stream(&self, _config: &SttConfig) -> Result<Box<dyn SttStream>> {
        if let Ok(script) = std::env::var("CONCH_TEST_SCRIPTED_STT_FINALS") {
            let finals: Vec<String> = script.split('|').map(|s| s.to_string()).collect();
            return Ok(Box::new(ScriptedLocalStream {
                finals: finals.into_iter().collect(),
            }));
        }
        Err(anyhow!("local STT backend is not implemented yet"))
    }
}

#[derive(Debug)]
struct ScriptedLocalStream {
    finals: std::collections::VecDeque<String>,
}

#[async_trait]
impl SttStream for ScriptedLocalStream {
    async fn send_frame(&mut self, _pcm: &[i16]) -> Result<()> {
        Ok(())
    }

    async fn end_of_utterance(&mut self) -> Result<()> {
        Ok(())
    }

    async fn next_event(&mut self) -> Option<TranscriptEvent> {
        let text = self.finals.pop_front()?;
        Some(TranscriptEvent::Final {
            text,
            words: vec![],
        })
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}
