use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "conch", version, about = "Voice interview CLI for project writing")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Fast rough brief using the haiku model
    Sketch {
        /// Topic: GitHub URL, project name, or freeform description
        topic: String,
    },
    /// Default interview brief using the sonnet model
    Talk {
        /// Topic: GitHub URL, project name, or freeform description
        topic: String,
    },
    /// Deep research brief using the opus model
    Chronicle {
        /// Topic: GitHub URL, project name, or freeform description
        topic: String,
    },
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
