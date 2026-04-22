use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "conch",
    version,
    about = "Voice interview CLI for project writing"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Args, Debug)]
pub struct InterviewArgs {
    pub topic: String,
    /// Disable TTS (text mode).
    #[arg(long)]
    pub no_tts: bool,
    /// STT backend override (deepgram | local).
    #[arg(long)]
    pub stt: Option<String>,
    /// TTS backend override (elevenlabs | local | text).
    #[arg(long)]
    pub tts: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Fast rough brief using the haiku model
    Sketch(InterviewArgs),
    /// Default interview brief using the sonnet model
    Talk(InterviewArgs),
    /// Deep research brief using the opus model
    Chronicle(InterviewArgs),
    /// Smoke test: runs sketch against a baked-in default topic
    Test,
    /// List past sessions
    Sessions,
    /// Re-export outputs from a past session
    Export {
        /// Session ID
        session_id: String,
        /// Destination directory
        #[arg(short, long)]
        out: Option<std::path::PathBuf>,
    },
}
