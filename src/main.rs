use clap::Parser;
use conch::cli::{Cli, Command};
use conch::model::Model;

const TEST_TOPIC: &str = "this repository";

fn apply_overrides(
    mut config: conch::config::Config,
    args: &conch::cli::InterviewArgs,
) -> anyhow::Result<conch::config::Config> {
    if let Some(stt) = &args.stt {
        config = config.with_stt_backend(stt.parse()?);
    }
    if args.no_tts {
        config = config.with_tts_backend(conch::config::TtsBackend::Text);
    } else if let Some(tts) = &args.tts {
        config = config.with_tts_backend(tts.parse()?);
    }
    Ok(config)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("conch=info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Sketch(args) => {
            let config = conch::config::Config::load()?;
            let config = apply_overrides(config, &args)?;
            conch::commands::interview::run(&config, Model::Haiku, &args.topic, None).await?;
        }
        Command::Talk(args) => {
            let config = conch::config::Config::load()?;
            let config = apply_overrides(config, &args)?;
            conch::commands::interview::run(&config, Model::Sonnet, &args.topic, None).await?;
        }
        Command::Chronicle(args) => {
            let config = conch::config::Config::load()?;
            let config = apply_overrides(config, &args)?;
            conch::commands::interview::run(&config, Model::Opus, &args.topic, None).await?;
        }
        Command::Test => {
            // SAFETY: single-threaded at this point (before orchestrator spawns tasks).
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var("CONCH_HEADLESS", "1");
            }
            let config = conch::config::Config::load()?;
            // `test` is headless/no-mic, so STT is always scripted (Local).
            // TTS defaults to Text for a silent smoke test; set CONCH_TTS=local
            // to hear piper produce audio.
            let tts = std::env::var("CONCH_TTS")
                .ok()
                .map(|s| s.parse())
                .transpose()?
                .unwrap_or(conch::config::TtsBackend::Text);
            let config = config
                .with_tts_backend(tts)
                .with_stt_backend(conch::config::SttBackend::Local);
            let cwd = std::env::current_dir()?;
            let label = cwd
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("test")
                .to_string();
            conch::commands::interview::run(&config, Model::Haiku, TEST_TOPIC, Some(&label))
                .await?;
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
