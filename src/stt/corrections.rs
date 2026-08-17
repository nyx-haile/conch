//! Transcription corrections: personal vocabulary that STT reliably mishears.
//!
//! Two complementary mechanisms:
//!
//! 1. **Preferred transcriptions** — `raw → canonical` examples that are
//!    formatted into prompt memory ([`CorrectionStore::prompt_memory_text`])
//!    and injected into the interview system prompt, so the LLM reads a
//!    garbled name the way the speaker meant it.
//! 2. **Replacement rules** — deterministic literal substitutions applied to
//!    the final STT text before the LLM ever sees it
//!    ([`CorrectionStore::apply_replacements`]).
//!
//! The split (persisted rules + a `prompt_memory` formatter) follows the shape
//! of pepper-x's `pepperx-corrections` crate (MIT, Jesse Vincent); no code is
//! copied. See `LICENSES/pepper-x-MIT.txt`.
//!
//! Storage is a hand-editable JSON document, by default at
//! `~/.conch/corrections.json` ([`crate::config::Config::corrections_file`]).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A `raw → preferred` transcription example.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreferredTranscription {
    /// What STT produced.
    pub raw: String,
    /// What the speaker actually said.
    pub preferred: String,
}

/// A literal substitution applied to final transcripts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplacementRule {
    /// Text to look for (matched ASCII-case-insensitively).
    pub from: String,
    /// Text to substitute in its place, verbatim.
    pub to: String,
}

/// On-disk shape of `corrections.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct CorrectionData {
    #[serde(default)]
    preferred: Vec<PreferredTranscription>,
    #[serde(default)]
    replacements: Vec<ReplacementRule>,
}

/// Persisted store of transcription corrections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectionStore {
    path: PathBuf,
    data: CorrectionData,
}

impl CorrectionStore {
    /// An empty store bound to `path`. Nothing is written until [`Self::save`].
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            data: CorrectionData::default(),
        }
    }

    /// Load the store at `path`. A missing file yields an empty store — a
    /// first-run user has no corrections and that is not an error.
    pub fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::new(path));
            }
            Err(e) => {
                return Err(e).with_context(|| format!("reading {}", path.display()));
            }
        };
        if raw.trim().is_empty() {
            return Ok(Self::new(path));
        }
        let data: CorrectionData = serde_json::from_str(&raw)
            .with_context(|| format!("parsing corrections JSON at {}", path.display()))?;
        Ok(Self { path, data })
    }

    /// Load the store, falling back to an empty one if the file is unreadable
    /// or malformed. Use on the interview hot path, where a broken
    /// corrections file must never block a session.
    pub fn load_or_default(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        match Self::load(path.clone()) {
            Ok(store) => store,
            Err(e) => {
                tracing::warn!(err = %e, path = %path.display(), "ignoring unreadable corrections file");
                Self::new(path)
            }
        }
    }

    /// Persist the store as pretty-printed JSON, creating parent dirs.
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(&self.data).context("serializing corrections")?;
        std::fs::write(&self.path, format!("{json}\n"))
            .with_context(|| format!("writing {}", self.path.display()))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_empty(&self) -> bool {
        self.data.preferred.is_empty() && self.data.replacements.is_empty()
    }

    pub fn preferred(&self) -> &[PreferredTranscription] {
        &self.data.preferred
    }

    pub fn replacements(&self) -> &[ReplacementRule] {
        &self.data.replacements
    }

    /// Record that `raw` should have been transcribed as `preferred`.
    /// Re-recording the same `raw` (case-insensitively) updates in place, so
    /// the store cannot accumulate contradictory duplicates.
    pub fn set_preferred_transcription(
        &mut self,
        raw: impl Into<String>,
        preferred: impl Into<String>,
    ) {
        let raw = raw.into();
        let preferred = preferred.into();
        match self
            .data
            .preferred
            .iter_mut()
            .find(|p| p.raw.eq_ignore_ascii_case(&raw))
        {
            Some(existing) => {
                existing.raw = raw;
                existing.preferred = preferred;
            }
            None => self
                .data
                .preferred
                .push(PreferredTranscription { raw, preferred }),
        }
    }

    /// Add (or update) a literal replacement rule. Empty `from` keys are
    /// ignored — they would match at every position.
    pub fn add_replacement_rule(&mut self, from: impl Into<String>, to: impl Into<String>) {
        let from = from.into();
        let to = to.into();
        if from.is_empty() {
            return;
        }
        match self
            .data
            .replacements
            .iter_mut()
            .find(|r| r.from.eq_ignore_ascii_case(&from))
        {
            Some(existing) => {
                existing.from = from;
                existing.to = to;
            }
            None => self.data.replacements.push(ReplacementRule { from, to }),
        }
    }

    /// Replacement rules ordered longest key first, so `"pepper x"` wins over
    /// `"pepper"` at the same position. Ties break on the key text for a
    /// stable, reproducible order.
    pub fn rules_longest_first(&self) -> Vec<&ReplacementRule> {
        let mut rules: Vec<&ReplacementRule> = self.data.replacements.iter().collect();
        rules.sort_by(|a, b| {
            b.from
                .chars()
                .count()
                .cmp(&a.from.chars().count())
                .then_with(|| a.from.cmp(&b.from))
        });
        rules
    }

    /// Apply every replacement rule to `text` in a single left-to-right pass.
    ///
    /// At each position the longest matching rule wins and the cursor jumps
    /// past the match, so substitutions never cascade into one another (a rule
    /// output is never re-scanned by another rule). Matching is
    /// ASCII-case-insensitive; the replacement is inserted verbatim.
    pub fn apply_replacements(&self, text: &str) -> String {
        let rules = self.rules_longest_first();
        if rules.is_empty() {
            return text.to_string();
        }
        let patterns: Vec<(Vec<char>, &str)> = rules
            .iter()
            .map(|r| (r.from.chars().collect::<Vec<char>>(), r.to.as_str()))
            .filter(|(pat, _)| !pat.is_empty())
            .collect();

        let haystack: Vec<char> = text.chars().collect();
        let mut out = String::with_capacity(text.len());
        let mut i = 0usize;
        while i < haystack.len() {
            let mut hit = None;
            for (pat, to) in &patterns {
                if i + pat.len() > haystack.len() {
                    continue;
                }
                if haystack[i..i + pat.len()]
                    .iter()
                    .zip(pat.iter())
                    .all(|(a, b)| a.eq_ignore_ascii_case(b))
                {
                    hit = Some((pat.len(), *to));
                    break;
                }
            }
            match hit {
                Some((len, to)) => {
                    out.push_str(to);
                    i += len;
                }
                None => {
                    out.push(haystack[i]);
                    i += 1;
                }
            }
        }
        out
    }

    /// Render the store as a prompt-memory block, or `None` when empty.
    ///
    /// Preferred examples keep insertion order; the vocabulary line follows
    /// the same longest-first order used by [`Self::apply_replacements`].
    pub fn prompt_memory_text(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }
        let mut out = String::new();
        out.push_str("Transcription memory:\n");

        if !self.data.preferred.is_empty() {
            out.push_str(
                "Speech-to-text mishears the following. If a transcript looks like the \
                 left side, read it as the right side.\n",
            );
            for p in &self.data.preferred {
                out.push_str(&format!(
                    "- heard \"{}\" -> meant \"{}\"\n",
                    p.raw, p.preferred
                ));
            }
        }

        let rules = self.rules_longest_first();
        if !rules.is_empty() {
            if !self.data.preferred.is_empty() {
                out.push('\n');
            }
            out.push_str(
                "These terms are already normalized in the transcript — spell them this way:\n",
            );
            for r in rules {
                out.push_str(&format!("- {}\n", r.to));
            }
        }

        Some(out.trim_end().to_string())
    }
}
