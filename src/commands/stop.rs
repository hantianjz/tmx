use crate::context::Context;
use crate::log;
use crate::tmux;
use anyhow::Result;

pub fn run(session_name: &str, _ctx: &Context) -> Result<()> {
    log::info(&format!("close command: session_name={}", session_name));

    // Check if tmux is installed
    if !tmux::is_installed() {
        log::error("tmux is not installed");
        anyhow::bail!("tmux is not installed");
    }

    // Check if session exists
    if !tmux::has_session(session_name)? {
        log::error(&format!("session '{}' does not exist", session_name));
        anyhow::bail!(
            "Session '{}' does not exist\nRun 'tmx running' to see active sessions.",
            session_name
        );
    }

    // Kill the session
    tmux::kill_session(session_name)?;
    log::info(&format!("session '{}' stopped", session_name));

    println!("✓ Session '{}' stopped", session_name);

    Ok(())
}
