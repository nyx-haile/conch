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

fn is_tui_command(cmd: &Command) -> bool {
    matches!(
        cmd,
        Command::Sketch(_) | Command::Talk(_) | Command::Chronicle(_)
    )
}

fn init_tracing(tui: bool) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("conch=info"));

    if tui {
        let logs_dir = dirs_home()
            .map(|h| h.join(".conch").join("logs"))
            .unwrap_or_else(|| std::path::PathBuf::from(".conch/logs"));
        if let Err(e) = std::fs::create_dir_all(&logs_dir) {
            // Fall back to stderr if we can't make the dir.
            eprintln!(
                "conch: could not create log dir {}: {e}",
                logs_dir.display()
            );
            tracing_subscriber::fmt().with_env_filter(filter).init();
            return None;
        }
        let file_appender = tracing_appender::rolling::daily(&logs_dir, "conch.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(non_blocking)
            .with_ansi(false)
            .init();
        Some(guard)
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
        None
    }
}

fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();
    let headless = std::env::var_os("CONCH_HEADLESS").is_some();
    let tui = is_tui_command(&cli.command) && !headless;
    let _log_guard = init_tracing(tui);

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
                // The Local STT backend is only useful in test mode when it
                // returns a scripted final transcript. Auto-drive expects
                // exactly one final ending the session via the wrap command.
                if std::env::var_os("CONCH_TEST_SCRIPTED_STT_FINALS").is_none() {
                    std::env::set_var("CONCH_TEST_SCRIPTED_STT_FINALS", "that's a wrap");
                }
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
