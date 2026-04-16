use crate::config::{Config, SttBackend, TtsBackend};
use crate::interview::history::ConversationLog;
use crate::interview::orchestrator::{LlmCaller, Orchestrator, OrchestratorConfig};
use crate::interview::tui::state::AppState;
use crate::interview::tui::UserEvent;
use crate::llm;
use crate::model::Model;
use crate::prep::run_prep;
use crate::session::Session;
use crate::stt::SpeechToText;
use crate::tts::TextToSpeech;
use anyhow::Context;
use chrono::Local;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

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

    // --- Build backends ---

    let stt: Arc<dyn SpeechToText> = match config.stt_backend() {
        SttBackend::Deepgram => {
            let key = config
                .deepgram_api_key()
                .context("DEEPGRAM_API_KEY required for deepgram STT backend")?;
            Arc::new(crate::stt::deepgram::DeepgramStt::production(key))
        }
        SttBackend::Local => Arc::new(crate::stt::local::LocalStt::new()),
    };

    let tts: Arc<dyn TextToSpeech> = match config.tts_backend() {
        TtsBackend::ElevenLabs => {
            let key = config
                .elevenlabs_api_key()
                .context("ELEVENLABS_API_KEY required for elevenlabs TTS backend")?;
            Arc::new(crate::tts::elevenlabs::ElevenLabsTts::production(
                key, "default",
            ))
        }
        TtsBackend::Text => {
            let (tts, _rx) = crate::tts::text::TextTts::new();
            Arc::new(tts)
        }
        TtsBackend::Local => Arc::new(crate::tts::local::LocalTts::new()),
    };

    let headless = std::env::var("CONCH_HEADLESS").is_ok();
    let sink: Box<dyn crate::audio::output::AudioSink> = if headless {
        Box::new(crate::audio::output::VecSink::new())
    } else {
        Box::new(
            crate::audio::output::RodioSink::new_default().context("opening audio output")?,
        )
    };

    let brand = std::fs::read_to_string(config.brand_file()).ok();
    let state = Arc::new(RwLock::new(AppState::new(
        format!("conch \u{00b7} {}", topic),
        brief.clone(),
    )));
    let (event_tx, event_rx) = mpsc::channel::<UserEvent>(32);

    let log =
        ConversationLog::new(session.conversation_path(), session.transcript_path())?;

    let orch_config = OrchestratorConfig {
        model: model_slug.clone(),
        brief: brief.clone(),
        brand,
        max_tokens: 1024,
        sample_rate: 16_000,
        fillers: None,
    };

    let llm_caller: Arc<dyn LlmCaller> = Arc::new(client);
    let orch = Orchestrator::new(
        llm_caller, stt, tts, sink, state.clone(), event_rx, orch_config,
    )
    .with_log(log);

    if headless {
        // Auto-drive: send mic toggle events to simulate a conversation
        let tx = event_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            let _ = tx.send(UserEvent::MicToggle).await;
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let _ = tx.send(UserEvent::MicToggle).await;
            // Scripted STT returns "that's a wrap" -> VoiceCommand end
        });
        let signal = orch.run().await?;
        println!("Interview ended: {signal:?}");
    } else {
        // Full TUI path
        run_tui(state, orch, event_tx).await?;
    }

    Ok(())
}

async fn run_tui(
    state: Arc<RwLock<AppState>>,
    orch: Orchestrator,
    event_tx: mpsc::Sender<UserEvent>,
) -> anyhow::Result<()> {
    use crate::interview::tui::widgets::render_frame;
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    };
    use ratatui::backend::CrosstermBackend;
    use ratatui::Terminal;

    enable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut term = Terminal::new(backend)?;

    let key_handle = tokio::spawn(crate::interview::tui::spawn_key_reader(event_tx));
    let orch_handle = tokio::spawn(orch.run());
    let start = std::time::Instant::now();

    loop {
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        let s = state.read().await;
        term.draw(|f| render_frame(f, &s, start.elapsed()))?;
        drop(s);
        if orch_handle.is_finished() {
            break;
        }
    }

    key_handle.abort();
    disable_raw_mode()?;
    crossterm::execute!(term.backend_mut(), LeaveAlternateScreen)?;
    orch_handle.await?.map(|_| ())
}
