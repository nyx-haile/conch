use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "conch", version, about = "Voice interview CLI for project writing")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Start a new interview session on a topic
    Talk {
        /// Topic: GitHub URL, project name, or freeform description
        topic: String,
    },
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
