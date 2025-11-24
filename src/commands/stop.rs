use crate::context::Context;
use crate::tmux;
use anyhow::Result;

pub fn run(session_name: &str, _ctx: &Context) -> Result<()> {
    // Check if tmux is installed
    if !tmux::is_installed() {
        anyhow::bail!("tmux is not installed");
    }

    // Check if session exists
    if !tmux::has_session(session_name)? {
        anyhow::bail!(
            "Session '{}' does not exist\nRun 'tmx running' to see active sessions.",
            session_name
        );
    }

    // Kill the session
    tmux::kill_session(session_name)?;

    println!("✓ Session '{}' stopped", session_name);

    Ok(())
}
