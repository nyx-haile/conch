use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Speaker {
    Conch,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub speaker: Speaker,
    pub text: String,
    pub timestamp_ms: u64,
    #[serde(default)]
    pub speculative_hit: bool,
    #[serde(default)]
    pub interrupted: bool,
    #[serde(default)]
    pub filler_played: Option<String>,
}

pub struct ConversationLog {
    json_path: PathBuf,
    md: File,
    turns: Vec<Turn>,
}

impl ConversationLog {
    pub fn new(json_path: impl AsRef<Path>, md_path: impl AsRef<Path>) -> Result<Self> {
        let md = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .private_file_mode()
            .open(md_path.as_ref())
            .with_context(|| format!("opening {}", md_path.as_ref().display()))?;
        set_private_file(&md, md_path.as_ref())?;
        Ok(Self {
            json_path: json_path.as_ref().to_path_buf(),
            md,
            turns: Vec::new(),
        })
    }

    pub fn append(&mut self, turn: Turn) -> Result<()> {
        let label = match turn.speaker {
            Speaker::Conch => "Conch",
            Speaker::User => "You",
        };
        let safe_text = turn.text.replace('\n', " ");
        writeln!(self.md, "**{}:** {}\n", label, safe_text).context("writing transcript.md")?;
        self.md.flush().context("flushing transcript.md")?;
        self.turns.push(turn);
        self.write_json()?;
        Ok(())
    }

    pub fn finalize(&mut self) -> Result<()> {
        self.write_json()
    }

    fn write_json(&self) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&self.turns).context("serializing conversation")?;
        let tmp = self.json_path.with_extension("json.tmp");
        crate::session::write_private(&tmp, bytes)
            .with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, &self.json_path).with_context(|| {
            format!("renaming {} -> {}", tmp.display(), self.json_path.display())
        })?;
        Ok(())
    }

    pub fn turns(&self) -> &[Turn] {
        &self.turns
    }
}

fn set_private_file(file: &File, path: &Path) -> Result<()> {
    #[cfg(unix)]
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("setting private permissions on {}", path.display()))?;
    Ok(())
}

trait PrivateFileMode {
    fn private_file_mode(&mut self) -> &mut Self;
}

impl PrivateFileMode for OpenOptions {
    fn private_file_mode(&mut self) -> &mut Self {
        #[cfg(unix)]
        self.mode(0o600);
        self
    }
}
