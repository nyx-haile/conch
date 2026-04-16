use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(date: &str, topic: &str) -> Self {
        let slug = slugify(topic);
        let slug = slug.chars().take(60).collect::<String>();
        let slug = slug.trim_end_matches('-').to_string();
        Self(format!("{}-{}", date, slug))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn directory(&self, sessions_root: &Path) -> PathBuf {
        sessions_root.join(&self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    id: SessionId,
    directory: PathBuf,
    topic: String,
}

impl Session {
    pub fn create(sessions_root: &Path, date: &str, topic: &str) -> anyhow::Result<Self> {
        let id = SessionId::new(date, topic);
        let directory = id.directory(sessions_root);
        std::fs::create_dir_all(&directory)?;
        Ok(Self {
            id,
            directory,
            topic: topic.to_string(),
        })
    }

    pub fn id(&self) -> &SessionId {
        &self.id
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn brief_path(&self) -> PathBuf {
        self.directory.join("brief.md")
    }

    pub fn raw_audio_path(&self) -> PathBuf {
        self.directory.join("raw_audio.wav")
    }

    pub fn transcript_path(&self) -> PathBuf {
        self.directory.join("transcript.md")
    }

    pub fn edited_path(&self) -> PathBuf {
        self.directory.join("edited.md")
    }

    pub fn conversation_path(&self) -> PathBuf {
        self.directory.join("conversation.json")
    }

    pub fn mic_wav_path(&self) -> PathBuf {
        self.directory.join("mic.wav")
    }

    pub fn tts_wav_path(&self) -> PathBuf {
        self.directory.join("tts.wav")
    }

    pub fn log_path(&self) -> PathBuf {
        self.directory.join("session.log")
    }
}

pub fn list_sessions(sessions_root: &Path) -> anyhow::Result<Vec<SessionId>> {
    if !sessions_root.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(sessions_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        if let Some(name) = entry.file_name().to_str() {
            ids.push(SessionId(name.to_string()));
        }
    }
    ids.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(ids)
}

fn slugify(input: &str) -> String {
    // Strip URL scheme (e.g. "https://", "http://") before slugifying
    let input = if let Some(rest) = input.strip_prefix("https://") {
        rest
    } else if let Some(rest) = input.strip_prefix("http://") {
        rest
    } else {
        input
    };

    let mut out = String::with_capacity(input.len());
    let mut last_dash = true;
    for c in input.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}
