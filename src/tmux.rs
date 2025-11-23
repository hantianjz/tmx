use anyhow::{Context, Result};
use std::process::{Command, Output};

/// Format a tmux window target (session:window_index)
fn window_target(session: &str, window_index: usize) -> String {
    format!("{}:{}", session, window_index)
}

/// Format a tmux pane target (session:window_index.pane_index)
fn pane_target(session: &str, window_index: usize, pane_index: usize) -> String {
    format!("{}:{}.{}", session, window_index, pane_index)
}

/// Check if debug mode is enabled
fn is_debug_mode() -> bool {
    std::env::var("TMX_DEBUG").is_ok()
}

/// Check if tmux is currently installed and available in PATH.
///
/// # Returns
/// `true` if tmux is installed, `false` otherwise.
pub fn is_installed() -> bool {
    Command::new("tmux")
        .arg("-V")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if we're currently running inside a tmux session.
///
/// # Returns
/// `true` if inside a tmux session (TMUX env var is set), `false` otherwise.
pub fn is_inside_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

/// Get the tmux base-index setting from global options.
///
/// The base-index determines the starting index for windows (typically 0 or 1).
///
/// # Returns
/// The base-index value, or 0 if not set or if an error occurs.
pub fn get_base_index() -> Result<usize> {
    static DEFAULT_BASE_INDEX: usize = 1;

    let output = Command::new("tmux")
        .args(["show-options", "-g", "base-index"])
        .output()
        .context("Failed to get tmux base-index")?;

    if !output.status.success() {
        return Ok(DEFAULT_BASE_INDEX); // Default to 1 if option not set
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output format: "base-index 1"
    let index = stdout
        .split_whitespace()
        .last()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_BASE_INDEX);

    Ok(index)
}

/// Check if a tmux session with the given name exists.
///
/// # Arguments
/// * `name` - The session name to check
///
/// # Returns
/// `Ok(true)` if the session exists, `Ok(false)` if it doesn't, or an error.
pub fn has_session(name: &str) -> Result<bool> {
    let output = Command::new("tmux")
        .args(["has-session", "-t", name])
        .output()
        .context("Failed to check session existence")?;

    Ok(output.status.success())
}

/// List all currently running tmux sessions.
///
/// # Returns
/// A vector of session names, or an empty vector if no sessions are running.
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

/// Get the current tmux session name (only works when inside tmux).
///
/// # Returns
/// The current session name, or an error if not inside tmux or command fails.
pub fn get_current_session() -> Result<String> {
    let output = execute_tmux(&["display-message", "-p", "#{session_name}"])?;
    let session = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(session)
}

/// Count the number of panes in a specific window.
///
/// # Arguments
/// * `session` - The session name
/// * `window_index` - The window index
///
/// # Returns
/// The number of panes in the window.
pub fn count_panes(session: &str, window_index: usize) -> Result<usize> {
    let target = window_target(session, window_index);
    let output = execute_tmux(&["list-panes", "-t", &target, "-F", "#{pane_index}"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.lines().count();
    Ok(count)
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

/// Split a window with specific size
pub fn split_window_with_size(
    session: &str,
    window_index: usize,
    horizontal: bool,
    size: Option<&str>,
    root: Option<&str>,
) -> Result<()> {
    let target = window_target(session, window_index);
    let split_flag = if horizontal { "-h" } else { "-v" };
    let mut args = vec!["split-window", "-t", &target, split_flag];

    // Add size parameter if specified
    if let Some(size_spec) = size {
        if size_spec.ends_with('%') {
            // Percentage size: use -p flag
            let percentage = size_spec.trim_end_matches('%');
            args.push("-p");
            args.push(percentage);
        } else {
            // Absolute size: use -l flag
            args.push("-l");
            args.push(size_spec);
        }
    }

    if let Some(dir) = root {
        args.push("-c");
        args.push(dir);
    }

    // Debug: print command being executed
    if is_debug_mode() {
        eprintln!("DEBUG: tmux {}", args.join(" "));
    }

    execute_tmux(&args)?;
    Ok(())
}

/// Split a window without specifying size (for refresh operations)
pub fn split_window(
    session: &str,
    window_index: usize,
    horizontal: bool,
    root: Option<&str>,
) -> Result<()> {
    let target = window_target(session, window_index);
    let split_flag = if horizontal { "-h" } else { "-v" };
    let mut args = vec!["split-window", "-t", &target, split_flag];

    if let Some(dir) = root {
        args.push("-c");
        args.push(dir);
    }

    if is_debug_mode() {
        eprintln!("DEBUG: tmux {}", args.join(" "));
    }

    execute_tmux(&args)?;
    Ok(())
}

/// Apply a layout to a window
pub fn select_layout(session: &str, window_index: usize, layout: &str) -> Result<()> {
    let target = window_target(session, window_index);

    // Debug: print layout command
    if is_debug_mode() {
        eprintln!("DEBUG: tmux select-layout -t {} {}", target, layout);
    }

    execute_tmux(&["select-layout", "-t", &target, layout])?;
    Ok(())
}

/// Send keys (commands) to a specific pane
pub fn send_keys(session: &str, window_index: usize, pane_index: usize, keys: &str) -> Result<()> {
    let target = pane_target(session, window_index, pane_index);
    execute_tmux(&["send-keys", "-t", &target, keys, "C-m"])?;
    Ok(())
}

/// Select a window
pub fn select_window(session: &str, window_index: usize) -> Result<()> {
    let target = window_target(session, window_index);
    execute_tmux(&["select-window", "-t", &target])?;
    Ok(())
}

/// Select a pane
pub fn select_pane(session: &str, window_index: usize, pane_index: usize) -> Result<()> {
    let target = pane_target(session, window_index, pane_index);
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
