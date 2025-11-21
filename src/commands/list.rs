use crate::config::Config;
use crate::tmux;
use anyhow::Result;

pub fn run() -> Result<()> {
    let config_path = Config::config_path()?;

    if !config_path.exists() {
        println!("No configuration file found at {}", config_path.display());
        println!("Run 'tmx init' to create one.");
        return Ok(());
    }

    // Load config
    let config = Config::load()?;

    // Get running sessions
    let running_sessions = tmux::list_sessions().unwrap_or_default();

    println!("Configured sessions:");
    if config.sessions.is_empty() {
        println!("  (none)");
    } else {
        for (id, session) in &config.sessions {
            let status = if running_sessions.contains(&session.name) {
                " (running)"
            } else {
                ""
            };
            println!("  {}{}", id, status);
        }
    }

    println!();
    println!("All running tmux sessions:");
    if running_sessions.is_empty() {
        println!("  (none)");
    } else {
        for session in running_sessions {
            println!("  {}", session);
        }
    }

    Ok(())
}

/// List only running sessions
pub fn run_running() -> Result<()> {
    let running_sessions = tmux::list_sessions().unwrap_or_default();

    println!("Running tmux sessions:");
    if running_sessions.is_empty() {
        println!("  (none)");
    } else {
        for session in running_sessions {
            println!("  {}", session);
        }
    }

    Ok(())
}

/// List only configured session names (for completions)
pub fn list_configured() -> Result<()> {
    let config = Config::load()?;
    for id in config.session_ids() {
        println!("{}", id);
    }
    Ok(())
}

/// List only running session names (for completions)
pub fn list_running() -> Result<()> {
    let running_sessions = tmux::list_sessions().unwrap_or_default();
    for session in running_sessions {
        println!("{}", session);
    }
    Ok(())
}
