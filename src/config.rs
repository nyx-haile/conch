use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Config {
    home: PathBuf,
    anthropic_api_key: Option<String>,
    github_token: Option<String>,
}

impl Config {
    pub fn with_home(home: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
            anthropic_api_key: None,
            github_token: None,
        }
    }

    pub fn conch_dir(&self) -> PathBuf {
        self.home.join(".conch")
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.conch_dir().join("sessions")
    }

    pub fn brand_file(&self) -> PathBuf {
        self.conch_dir().join("brand.md")
    }

    pub fn anthropic_api_key(&self) -> Option<&str> {
        self.anthropic_api_key.as_deref()
    }

    pub fn github_token(&self) -> Option<&str> {
        self.github_token.as_deref()
    }
}
