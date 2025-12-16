use crate::context::Context;
use crate::tmux;
use anyhow::Result;

pub fn run(ctx: &Context) -> Result<()> {
    // Get config from context (lazy-loaded)
    let config = ctx.config()?;

    // Get running sessions
    let running_sessions = tmux::list_sessions().unwrap_or_default();

    // Collect configured session names to filter from running list
    let configured_session_names: std::collections::HashSet<_> = config
        .sessions
        .values()
        .map(|s| s.name.clone())
        .collect();

    // Filter out configured sessions from running sessions
    let other_running: Vec<_> = running_sessions
        .iter()
        .filter(|s| !configured_session_names.contains(*s))
        .collect();

    // Only show configured sessions if no sessions are running
    if running_sessions.is_empty() {
        println!("Configured sessions:");
        let session_ids = config.session_ids();
        if session_ids.is_empty() {
            println!("  (none)");
        } else {
            for id in session_ids {
                println!("  {}", id);
            }
        }
        println!();
    }

    println!("Running tmux sessions:");
    if running_sessions.is_empty() {
        println!("  (none)");
    } else {
        // Show configured sessions that are running
        let session_ids = config.session_ids();
        for id in &session_ids {
            if let Some(session) = config.sessions.get(id) {
                if running_sessions.contains(&session.name) {
                    println!("  {} (c)", id);
                }
            }
        }
        // Show other running sessions (not configured)
        for session in other_running {
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
