use crate::config::Config;
use crate::session::list_sessions;

pub fn run(config: &Config) -> anyhow::Result<()> {
    let sessions = list_sessions(&config.sessions_dir())?;
    if sessions.is_empty() {
        println!("No sessions yet. Start one with `conch talk <topic>`.");
        return Ok(());
    }
    for id in sessions {
        println!("{}", id.as_str());
    }
    Ok(())
}
