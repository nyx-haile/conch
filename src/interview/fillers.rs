use crate::audio::wav::WavSessionWriter;
use crate::tts::TextToSpeech;
use anyhow::{Context, Result};
use hound::WavReader;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const DEFAULT_INTERRUPT_FILLERS: &[&str] = &[
    "mm?",
    "oh—",
    "sure, go ahead",
    "wait, hm",
    "mhm?",
    "right—",
    "yeah?",
    "—oh",
];

pub const DEFAULT_THINKING_FILLERS: &[&str] = &[
    "hmm…",
    "let me think about that",
    "interesting, okay",
    "right, so…",
    "oh, that's a good one",
    "mmm, yeah",
    "okay, okay",
    "so, thinking about that for a sec",
    "yeah, yeah",
    "hm, how to put this",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillerCategory {
    Interrupt,
    Thinking,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Manifest {
    entries: HashMap<String, String>,
    categories: HashMap<String, String>,
}

pub struct FillerCache {
    dir: PathBuf,
    manifest: Manifest,
    decoded: HashMap<String, Vec<i16>>,
    sample_rate: u32,
}

impl FillerCache {
    pub fn load_or_new(root: &Path, backend: &str, voice_id: &str) -> Result<Self> {
        let dir = root.join(backend).join(voice_id);
        std::fs::create_dir_all(&dir).context("creating filler cache dir")?;
        let manifest_path = dir.join("manifest.json");
        let manifest: Manifest = if manifest_path.exists() {
            let bytes = std::fs::read(&manifest_path).context("reading manifest")?;
            serde_json::from_slice(&bytes).unwrap_or_default()
        } else {
            Manifest::default()
        };

        let mut decoded: HashMap<String, Vec<i16>> = HashMap::new();
        let mut sample_rate: u32 = 24_000;
        for (text, file) in &manifest.entries {
            let p = dir.join(file);
            if let Ok(reader) = WavReader::open(&p) {
                sample_rate = reader.spec().sample_rate;
                let samples: Vec<i16> = reader
                    .into_samples::<i16>()
                    .filter_map(|r| r.ok())
                    .collect();
                decoded.insert(text.clone(), samples);
            }
        }

        Ok(Self {
            dir,
            manifest,
            decoded,
            sample_rate,
        })
    }

    pub fn sync<'a, T: TextToSpeech + 'a>(
        &'a mut self,
        tts: &'a T,
    ) -> impl std::future::Future<Output = Result<()>> + 'a {
        let missing_interrupt: Vec<&str> = DEFAULT_INTERRUPT_FILLERS
            .iter()
            .copied()
            .filter(|t| !self.manifest.entries.contains_key(*t))
            .collect();
        let missing_thinking: Vec<&str> = DEFAULT_THINKING_FILLERS
            .iter()
            .copied()
            .filter(|t| !self.manifest.entries.contains_key(*t))
            .collect();

        let mut all: Vec<&str> = Vec::new();
        all.extend_from_slice(&missing_interrupt);
        all.extend_from_slice(&missing_thinking);

        async move {
            if all.is_empty() {
                return Ok(());
            }

            let pcms = tts
                .synthesize_batch(&all)
                .await
                .context("batch synthesize fillers")?;

            for (text, pcm) in all.iter().zip(pcms.into_iter()) {
                let file = format!("{}.wav", hash_hex(text));
                let path = self.dir.join(&file);
                let mut w = WavSessionWriter::create(&path, self.sample_rate, 1)?;
                w.write_i16(&pcm)?;
                drop(w);
                self.manifest
                    .entries
                    .insert(text.to_string(), file.clone());
                let category = if DEFAULT_INTERRUPT_FILLERS.contains(text) {
                    "interrupt"
                } else {
                    "thinking"
                };
                self.manifest
                    .categories
                    .insert(text.to_string(), category.to_string());
                self.decoded.insert(text.to_string(), pcm);
            }
            let manifest_path = self.dir.join("manifest.json");
            std::fs::write(
                &manifest_path,
                serde_json::to_vec_pretty(&self.manifest)?,
            )
            .context("writing manifest")?;
            Ok(())
        }
    }

    pub fn pick_random(&self, category: FillerCategory) -> Option<Vec<i16>> {
        let candidates: Vec<&Vec<i16>> = self
            .manifest
            .categories
            .iter()
            .filter(|(_, c)| match category {
                FillerCategory::Interrupt => c.as_str() == "interrupt",
                FillerCategory::Thinking => c.as_str() == "thinking",
            })
            .filter_map(|(text, _)| self.decoded.get(text))
            .collect();
        let mut rng = rand::thread_rng();
        candidates.choose(&mut rng).map(|v| (*v).clone())
    }
}

fn hash_hex(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:016x}", h.finish())
}
