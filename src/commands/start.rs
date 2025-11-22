use crate::config::Config;
use crate::session;
use crate::tmux;
use anyhow::Result;

/// Attach to or switch to a tmux session depending on context.
///
/// If already inside tmux, switches the client to the target session.
/// Otherwise, attaches to the session from outside tmux.
fn attach_or_switch(session_name: &str) -> Result<()> {
    if tmux::is_inside_tmux() {
        tmux::switch_client(session_name)
    } else {
        tmux::attach_session(session_name)
    }
}

/// Start or attach to a configured tmux session.
///
/// If the session doesn't exist, it will be created from the configuration.
/// If it already exists, we'll attach to it.
///
/// # Arguments
/// * `session_id` - The session ID from the configuration file
pub fn run(session_id: &str) -> Result<()> {
    // Check if tmux is installed
    if !tmux::is_installed() {
        anyhow::bail!("tmux is not installed");
    }

    // Load config
    let config = Config::load()?;

    // Find the session
    let session = config
        .get_session(session_id)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Session '{}' not found in configuration\nAvailable sessions: {}",
                session_id,
                config.session_ids().join(", ")
            )
        })?;

    let session_name = &session.name;

    // Check if session already exists
    if tmux::has_session(session_name)? {
        println!("Session '{}' already exists. Attaching...", session_name);
        attach_or_switch(session_name)?;
    } else {
        // Create the session
        session::create_session(session)?;
        // Attach to the newly created session
        attach_or_switch(session_name)?;
    }

    Ok(())
}
