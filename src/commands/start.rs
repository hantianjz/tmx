use crate::config::Config;
use crate::session;
use crate::tmux;
use anyhow::Result;

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

        // Check if we're inside tmux
        if tmux::is_inside_tmux() {
            // Switch to the session
            tmux::switch_client(session_name)?;
        } else {
            // Attach to the session
            tmux::attach_session(session_name)?;
        }
    } else {
        // Create the session
        session::create_session(session)?;

        // Attach to the newly created session
        if tmux::is_inside_tmux() {
            tmux::switch_client(session_name)?;
        } else {
            tmux::attach_session(session_name)?;
        }
    }

    Ok(())
}
