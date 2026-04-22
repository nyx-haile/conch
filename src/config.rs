use crate::provider::Provider;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SttBackend {
    Deepgram,
    Local,
}

impl FromStr for SttBackend {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "deepgram" => Ok(Self::Deepgram),
            "local" => Ok(Self::Local),
            other => Err(anyhow!(
                "unknown STT backend {:?}; valid: deepgram, local",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsBackend {
    ElevenLabs,
    Local,
    Text,
}

impl FromStr for TtsBackend {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "elevenlabs" => Ok(Self::ElevenLabs),
            "local" => Ok(Self::Local),
            "text" => Ok(Self::Text),
            other => Err(anyhow!(
                "unknown TTS backend {:?}; valid: elevenlabs, local, text",
                other
            )),
        }
    }
}

/// Rachel — a safe, widely-available default voice. Override via
/// `CONCH_ELEVEN_VOICE_ID` once you've picked a voice from VoiceLab.
const DEFAULT_ELEVEN_VOICE_ID: &str = "21m00Tcm4TlvDq8ikWAM";

#[derive(Debug, Clone)]
pub struct Config {
    home: PathBuf,
    openrouter_api_key: Option<String>,
    anthropic_api_key: Option<String>,
    deepgram_api_key: Option<String>,
    elevenlabs_api_key: Option<String>,
    elevenlabs_voice_id: Option<String>,
    github_token: Option<String>,
    provider: Option<Provider>,
    stt_backend: SttBackend,
    tts_backend: TtsBackend,
}

impl Config {
    pub fn with_home(home: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
            openrouter_api_key: None,
            anthropic_api_key: None,
            deepgram_api_key: None,
            elevenlabs_api_key: None,
            elevenlabs_voice_id: None,
            github_token: None,
            provider: None,
            stt_backend: SttBackend::Deepgram,
            tts_backend: TtsBackend::Local,
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
    pub fn fillers_dir(&self) -> PathBuf {
        self.conch_dir().join("fillers")
    }
    pub fn parakeet_model_dir(&self) -> PathBuf {
        if let Ok(p) = std::env::var("CONCH_PARAKEET_MODEL_DIR") {
            return PathBuf::from(p);
        }
        self.conch_dir()
            .join("models")
            .join("parakeet-nemotron-streaming-en-0.6b")
    }

    pub fn openrouter_api_key(&self) -> Option<&str> {
        self.openrouter_api_key.as_deref()
    }
    pub fn anthropic_api_key(&self) -> Option<&str> {
        self.anthropic_api_key.as_deref()
    }
    pub fn deepgram_api_key(&self) -> Option<&str> {
        self.deepgram_api_key.as_deref()
    }
    pub fn elevenlabs_api_key(&self) -> Option<&str> {
        self.elevenlabs_api_key.as_deref()
    }
    pub fn elevenlabs_voice_id(&self) -> &str {
        self.elevenlabs_voice_id
            .as_deref()
            .unwrap_or(DEFAULT_ELEVEN_VOICE_ID)
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

    pub fn stt_backend(&self) -> SttBackend {
        self.stt_backend
    }
    pub fn tts_backend(&self) -> TtsBackend {
        self.tts_backend
    }
    pub fn with_stt_backend(mut self, b: SttBackend) -> Self {
        self.stt_backend = b;
        self
    }
    pub fn with_tts_backend(mut self, b: TtsBackend) -> Self {
        self.tts_backend = b;
        self
    }

    pub fn from_env_map(
        home: &Path,
        env: &std::collections::HashMap<String, String>,
    ) -> Result<Self> {
        let provider = match env.get("CONCH_PROVIDER") {
            Some(v) => Some(v.parse::<Provider>()?),
            None => None,
        };
        let stt_backend = match env.get("CONCH_STT") {
            Some(v) => v.parse()?,
            None => SttBackend::Deepgram,
        };
        let tts_backend = match env.get("CONCH_TTS") {
            Some(v) => v.parse()?,
            None => TtsBackend::Local,
        };
        Ok(Self {
            home: home.to_path_buf(),
            openrouter_api_key: env.get("OPENROUTER_API_KEY").cloned(),
            anthropic_api_key: env.get("ANTHROPIC_API_KEY").cloned(),
            deepgram_api_key: env.get("DEEPGRAM_API_KEY").cloned(),
            elevenlabs_api_key: env.get("ELEVENLABS_API_KEY").cloned(),
            elevenlabs_voice_id: env.get("CONCH_ELEVEN_VOICE_ID").cloned(),
            github_token: env.get("GITHUB_TOKEN").cloned(),
            provider,
            stt_backend,
            tts_backend,
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
