use crate::context::Context;
use crate::tmux;
use anyhow::Result;

pub fn run(ctx: &Context) -> Result<()> {
    // Get config from context (lazy-loaded)
    let config = ctx.config()?;

    // Get running sessions
    let running_sessions = tmux::list_sessions().unwrap_or_default();

    println!("Configured sessions:");
    let session_ids = config.session_ids();
    if session_ids.is_empty() {
        println!("  (none)");
    } else {
        for id in session_ids {
            if let Some(session) = config.sessions.get(&id) {
                let status = if running_sessions.contains(&session.name) {
                    " (running)"
                } else {
                    ""
                };
                println!("  {}{}", id, status);
            }
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

/// List only configured session names (for completions)
pub fn list_configured(ctx: &Context) -> Result<()> {
    let config = ctx.config()?;
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
