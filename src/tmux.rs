use anyhow::{Context, Result};
use std::process::{Command, Output};

/// Check if tmux is installed
pub fn is_installed() -> bool {
    Command::new("tmux")
        .arg("-V")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if we're currently inside a tmux session
pub fn is_inside_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

/// Get the base-index setting (default 0)
pub fn get_base_index() -> Result<usize> {
    let output = Command::new("tmux")
        .args(["show-options", "-g", "base-index"])
        .output()
        .context("Failed to get tmux base-index")?;

    if !output.status.success() {
        return Ok(0); // Default to 0 if option not set
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output format: "base-index 1"
    let index = stdout
        .split_whitespace()
        .last()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    Ok(index)
}

/// Check if a session exists
pub fn has_session(name: &str) -> Result<bool> {
    let output = Command::new("tmux")
        .args(["has-session", "-t", name])
        .output()
        .context("Failed to check session existence")?;

    Ok(output.status.success())
}

/// List all running tmux sessions
pub fn list_sessions() -> Result<Vec<String>> {
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output()
        .context("Failed to list tmux sessions")?;

    if !output.status.success() {
        // No sessions running
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let sessions: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();

    Ok(sessions)
}

/// Create a new tmux session
pub fn new_session(name: &str, window_name: &str, root: Option<&str>) -> Result<()> {
    let mut args = vec!["new-session", "-d", "-s", name, "-n", window_name];

    if let Some(dir) = root {
        args.push("-c");
        args.push(dir);
    }

    execute_tmux(&args)?;
    Ok(())
}

/// Create a new window in a session
pub fn new_window(session: &str, window_name: &str, root: Option<&str>) -> Result<()> {
    let target = format!("{}:", session);
    let mut args = vec!["new-window", "-t", &target, "-n", window_name];

    if let Some(dir) = root {
        args.push("-c");
        args.push(dir);
    }

    execute_tmux(&args)?;
    Ok(())
}

/// Split a window to create a new pane
pub fn split_window(
    session: &str,
    window_index: usize,
    horizontal: bool,
    root: Option<&str>,
) -> Result<()> {
    let target = format!("{}:{}", session, window_index);
    let split_flag = if horizontal { "-h" } else { "-v" };
    let mut args = vec!["split-window", "-t", &target, split_flag];

    if let Some(dir) = root {
        args.push("-c");
        args.push(dir);
    }

    execute_tmux(&args)?;
    Ok(())
}

/// Apply a layout to a window
pub fn select_layout(session: &str, window_index: usize, layout: &str) -> Result<()> {
    let target = format!("{}:{}", session, window_index);
    execute_tmux(&["select-layout", "-t", &target, layout])?;
    Ok(())
}

/// Send keys (commands) to a specific pane
pub fn send_keys(session: &str, window_index: usize, pane_index: usize, keys: &str) -> Result<()> {
    let target = format!("{}:{}.{}", session, window_index, pane_index);
    execute_tmux(&["send-keys", "-t", &target, keys, "C-m"])?;
    Ok(())
}

/// Select a window
pub fn select_window(session: &str, window_index: usize) -> Result<()> {
    let target = format!("{}:{}", session, window_index);
    execute_tmux(&["select-window", "-t", &target])?;
    Ok(())
}

/// Select a pane
pub fn select_pane(session: &str, window_index: usize, pane_index: usize) -> Result<()> {
    let target = format!("{}:{}.{}", session, window_index, pane_index);
    execute_tmux(&["select-pane", "-t", &target])?;
    Ok(())
}

/// Attach to a session
pub fn attach_session(name: &str) -> Result<()> {
    execute_tmux_interactive(&["attach-session", "-t", name])?;
    Ok(())
}

/// Switch to a session (when already inside tmux)
pub fn switch_client(name: &str) -> Result<()> {
    execute_tmux(&["switch-client", "-t", name])?;
    Ok(())
}

/// Kill a session
pub fn kill_session(name: &str) -> Result<()> {
    execute_tmux(&["kill-session", "-t", name])?;
    Ok(())
}

/// Execute a tmux command
fn execute_tmux(args: &[&str]) -> Result<Output> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .context("Failed to execute tmux command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("tmux command failed: {}", stderr.trim());
    }

    Ok(output)
}

/// Execute a tmux command interactively (for attach)
fn execute_tmux_interactive(args: &[&str]) -> Result<()> {
    let status = Command::new("tmux")
        .args(args)
        .status()
        .context("Failed to execute tmux command")?;

    if !status.success() {
        anyhow::bail!("tmux command failed with status: {}", status);
    }

    Ok(())
}
