use crate::config::Config;
use crate::llm;
use crate::model::Model;
use crate::prep::run_prep;
use crate::session::Session;
use anyhow::Context;
use chrono::Local;

pub async fn run(
    config: &Config,
    model: Model,
    topic: &str,
    session_label: Option<&str>,
) -> anyhow::Result<()> {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let label = session_label.unwrap_or(topic);
    let session = Session::create(&config.sessions_dir(), &date, label)
        .context("creating session directory")?;

    let provider = config.provider()?;
    let (client, model_slug) = llm::resolve(config, provider, model)?;

    println!("Session: {}", session.id().as_str());
    println!("Model: {}", model_slug);
    println!("Researching topic...");

    let brief = run_prep(config, &client, &model_slug, topic)
        .await
        .context("prep stage failed")?;
    std::fs::write(session.brief_path(), &brief).context("writing brief.md")?;

    println!("Brief written to {}", session.brief_path().display());
    println!("\nNote: interview, process, and edit stages are not yet implemented.");
    Ok(())
}
