use crate::context::Context;
use crate::session;
use crate::tmux;
use anyhow::Result;

/// Attach to or switch to a tmux session depending on context.
///
/// If already inside tmux, switches the client to the target session.
/// Otherwise, attaches to the session from outside tmux.
fn attach_or_switch(session_name: &str, ctx: &Context) -> Result<()> {
    if ctx.is_inside_tmux {
        tmux::switch_client(session_name)
    } else {
        tmux::attach_session(session_name)
    }
}

/// Start or attach to a tmux session.
///
/// If the session already exists in tmux, we'll attach to it directly.
/// If not, we'll look it up in the configuration and create it.
///
/// # Arguments
/// * `session_id` - The session ID/name to attach to or create
/// * `ctx` - Shared context containing configuration and state
pub fn run(session_id: &str, ctx: &Context) -> Result<()> {
    // Check if tmux is installed
    if !tmux::is_installed() {
        anyhow::bail!("tmux is not installed");
    }

    // First, check if a session with this name already exists in tmux
    // This allows attaching to any existing session, even if not in config
    if tmux::has_session(session_id)? {
        println!("Attaching to existing session '{}'...", session_id);
        return attach_or_switch(session_id, ctx);
    }

    // Session doesn't exist, so we need to create it from configuration
    let config = ctx.config()?;

    // Find the session in config
    let session = config.get_session(session_id).ok_or_else(|| {
        anyhow::anyhow!(
            "Session '{}' not found in configuration\nAvailable sessions: {}",
            session_id,
            config.session_ids().join(", ")
        )
    })?;

    let session_name = &session.name;

    // Double-check if session exists with the configured name (may differ from session_id)
    if tmux::has_session(session_name)? {
        println!("Attaching to existing session '{}'...", session_name);
        attach_or_switch(session_name, ctx)?;
    } else {
        // Create the session
        session::create_session(session, ctx)?;
        // Attach to the newly created session
        attach_or_switch(session_name, ctx)?;
    }

    Ok(())
}
