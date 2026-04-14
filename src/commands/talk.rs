use crate::config::Config;
use crate::prep::run_prep;
use crate::session::Session;
use anyhow::Context;
use chrono::Local;

pub async fn run(config: &Config, topic: &str) -> anyhow::Result<()> {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let session = Session::create(&config.sessions_dir(), &date, topic)
        .context("creating session directory")?;

    println!("Session: {}", session.id().as_str());
    println!("Researching topic...");

    let brief = run_prep(config, topic).await.context("prep stage failed")?;
    std::fs::write(session.brief_path(), &brief)
        .context("writing brief.md")?;

    println!("Brief written to {}", session.brief_path().display());
    println!("\nNote: interview, process, and edit stages are not yet implemented.");
    Ok(())
}
