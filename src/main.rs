use clap::Parser;
use conch::cli::{Cli, Command};

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
        Command::Talk { topic } => {
            let config = conch::config::Config::load()?;
            conch::commands::talk::run(&config, &topic).await?;
        }
        Command::Sessions => {
            let config = conch::config::Config::load()?;
            conch::commands::sessions::run(&config)?;
        }
        Command::Export { session_id, out } => {
            println!("export: {} -> {:?}", session_id, out);
        }
    }
    Ok(())
}
