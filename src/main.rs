use clap::Parser;
use conch::cli::{Cli, Command};
use conch::model::Model;

const TEST_TOPIC: &str = "conch";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("conch=info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Sketch { topic } => {
            let config = conch::config::Config::load()?;
            conch::commands::interview::run(&config, Model::Haiku, &topic).await?;
        }
        Command::Talk { topic } => {
            let config = conch::config::Config::load()?;
            conch::commands::interview::run(&config, Model::Sonnet, &topic).await?;
        }
        Command::Chronicle { topic } => {
            let config = conch::config::Config::load()?;
            conch::commands::interview::run(&config, Model::Opus, &topic).await?;
        }
        Command::Test => {
            let config = conch::config::Config::load()?;
            conch::commands::interview::run(&config, Model::Haiku, TEST_TOPIC).await?;
        }
        Command::Sessions => {
            let config = conch::config::Config::load()?;
            conch::commands::sessions::run(&config)?;
        }
        Command::Export { session_id, out } => {
            let config = conch::config::Config::load()?;
            conch::commands::export::run(&config, &session_id, out)?;
        }
    }
    Ok(())
}
