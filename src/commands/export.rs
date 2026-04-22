use crate::config::Config;
use anyhow::{anyhow, Context};
use std::path::PathBuf;

pub fn run(config: &Config, session_id: &str, out: Option<PathBuf>) -> anyhow::Result<()> {
    let session_dir = config.sessions_dir().join(session_id);
    if !session_dir.exists() {
        return Err(anyhow!("session not found: {}", session_id));
    }

    let out = out.unwrap_or_else(|| PathBuf::from(format!("./{}", session_id)));
    std::fs::create_dir_all(&out).context("creating output dir")?;

    let candidates = [
        "brief.md",
        "raw_audio.wav",
        "transcript.md",
        "edited.md",
        "edited_audio.wav",
    ];
    let mut copied = 0;
    for name in candidates {
        let src = session_dir.join(name);
        if src.exists() {
            std::fs::copy(&src, out.join(name)).with_context(|| format!("copying {}", name))?;
            copied += 1;
        }
    }

    println!("Exported {} file(s) to {}", copied, out.display());
    Ok(())
}
