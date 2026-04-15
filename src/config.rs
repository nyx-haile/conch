use crate::provider::Provider;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Config {
    home: PathBuf,
    openrouter_api_key: Option<String>,
    anthropic_api_key: Option<String>,
    github_token: Option<String>,
    provider: Option<Provider>,
}

impl Config {
    pub fn with_home(home: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
            openrouter_api_key: None,
            anthropic_api_key: None,
            github_token: None,
            provider: None,
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

    pub fn openrouter_api_key(&self) -> Option<&str> {
        self.openrouter_api_key.as_deref()
    }

    pub fn anthropic_api_key(&self) -> Option<&str> {
        self.anthropic_api_key.as_deref()
    }

    pub fn github_token(&self) -> Option<&str> {
        self.github_token.as_deref()
    }

    pub fn provider(&self) -> Result<Provider> {
        self.provider.ok_or_else(|| {
            anyhow!(
                "CONCH_PROVIDER is not set. Set it to one of: anthropic, deepseek, meta, google"
            )
        })
    }

    pub fn from_env_map(
        home: &Path,
        env: &std::collections::HashMap<String, String>,
    ) -> Result<Self> {
        let provider = match env.get("CONCH_PROVIDER") {
            Some(v) => Some(v.parse::<Provider>()?),
            None => None,
        };
        Ok(Self {
            home: home.to_path_buf(),
            openrouter_api_key: env.get("OPENROUTER_API_KEY").cloned(),
            anthropic_api_key: env.get("ANTHROPIC_API_KEY").cloned(),
            github_token: env.get("GITHUB_TOKEN").cloned(),
            provider,
        })
    }

    pub fn load() -> Result<Self> {
        let home = directories::UserDirs::new()
            .ok_or_else(|| anyhow!("could not determine user home directory"))?
            .home_dir()
            .to_path_buf();
        let env: std::collections::HashMap<String, String> = std::env::vars().collect();
        Self::from_env_map(&home, &env)
    }
}
