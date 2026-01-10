use anyhow::{Context, Result};
use std::process::{Command, Output};

use crate::log;

/// Format a tmux window target (session:window_index)
fn window_target(session: &str, window_index: usize) -> String {
    let sanitized = sanitize_session_name(session);
    format!("{}:{}", sanitized, window_index)
}

/// Format a tmux pane target (session:window_index.pane_index)
fn pane_target(session: &str, window_index: usize, pane_index: usize) -> String {
    let sanitized = sanitize_session_name(session);
    format!("{}:{}.{}", sanitized, window_index, pane_index)
}

/// Sanitize a session name to be compatible with tmux.
///
/// Tmux replaces certain special characters (like dots and colons) with underscores
/// because they're used as separators in target notation (session:window.pane).
/// This function replicates that behavior to ensure consistency.
///
/// # Arguments
/// * `name` - The original session name
///
/// # Returns
/// The sanitized session name with special characters replaced by underscores
pub fn sanitize_session_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            // Tmux uses these as separators, so they must be replaced
            '.' | ':' => '_',
            // Also replace other potentially problematic characters
            ' ' | '\t' | '\n' => '_',
            _ => c,
        })
        .collect()
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
    let sanitized = sanitize_session_name(name);
    let output = Command::new("tmux")
        .args(["has-session", "-t", &sanitized])
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

/// Get window dimensions (width and height in cells/lines)
///
/// # Arguments
/// * `session` - The session name
/// * `window_index` - The window index
///
/// # Returns
/// A tuple of (width, height) in cells/lines
pub fn get_window_dimensions(session: &str, window_index: usize) -> Result<(usize, usize)> {
    let target = window_target(session, window_index);
    let output = execute_tmux(&[
        "display-message",
        "-t",
        &target,
        "-p",
        "#{window_width} #{window_height}",
    ])?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.split_whitespace().collect();

    if parts.len() != 2 {
        anyhow::bail!("Failed to parse window dimensions: {}", stdout);
    }

    let width = parts[0].parse::<usize>()
        .context("Failed to parse window width")?;
    let height = parts[1].parse::<usize>()
        .context("Failed to parse window height")?;

    Ok((width, height))
}

/// Create a new tmux session
pub fn new_session(name: &str, window_name: &str, root: Option<&str>) -> Result<()> {
    let sanitized = sanitize_session_name(name);
    let mut args = vec!["new-session", "-d", "-s", &sanitized, "-n", window_name];

    if let Some(dir) = root {
        args.push("-c");
        args.push(dir);
    }

    execute_tmux(&args)?;
    Ok(())
}

/// Create a new window in a session
pub fn new_window(session: &str, window_name: &str, root: Option<&str>) -> Result<()> {
    let sanitized = sanitize_session_name(session);
    let target = format!("{}:", sanitized);
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
    verbose: bool,
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
    if verbose {
        eprintln!("tmux {}", args.join(" "));
    }

    execute_tmux(&args)?;
    Ok(())
}

/// Apply a layout to a window
pub fn select_layout(
    session: &str,
    window_index: usize,
    layout: &str,
    verbose: bool,
) -> Result<()> {
    let target = window_target(session, window_index);

    // Debug: print layout command
    if verbose {
        eprintln!("tmux select-layout -t {} {}", target, layout);
    }

    execute_tmux(&["select-layout", "-t", &target, layout])?;
    Ok(())
}

/// Resize a specific pane to an absolute size
///
/// # Arguments
/// * `session` - The session name
/// * `window_index` - The window index
/// * `pane_index` - The pane index
/// * `size` - Absolute size in cells/lines (already calculated from percentage if needed)
/// * `is_horizontal` - True for horizontal split (resize width), false for vertical (resize height)
/// * `verbose` - Whether to print debug info
pub fn resize_pane(
    session: &str,
    window_index: usize,
    pane_index: usize,
    size: usize,
    is_horizontal: bool,
    verbose: bool,
) -> Result<()> {
    let target = pane_target(session, window_index, pane_index);
    let size_str = size.to_string();

    // For horizontal splits, we resize width (-x)
    // For vertical splits, we resize height (-y)
    let dimension_flag = if is_horizontal { "-x" } else { "-y" };

    let args = vec!["resize-pane", "-t", &target, dimension_flag, &size_str];

    if verbose {
        eprintln!("tmux {}", args.join(" "));
    }

    execute_tmux(&args)?;
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
    let sanitized = sanitize_session_name(name);
    execute_tmux_interactive(&["attach-session", "-t", &sanitized])?;
    Ok(())
}

/// Switch to a session (when already inside tmux)
pub fn switch_client(name: &str) -> Result<()> {
    let sanitized = sanitize_session_name(name);
    execute_tmux(&["switch-client", "-t", &sanitized])?;
    Ok(())
}

/// Kill a session
pub fn kill_session(name: &str) -> Result<()> {
    let sanitized = sanitize_session_name(name);
    execute_tmux(&["kill-session", "-t", &sanitized])?;
    Ok(())
}

/// Execute a tmux command
fn execute_tmux(args: &[&str]) -> Result<Output> {
    log::debug(&format!("tmux {}", args.join(" ")));

    let output = Command::new("tmux")
        .args(args)
        .output()
        .context("Failed to execute tmux command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error(&format!("tmux {} -> FAILED: {}", args.join(" "), stderr.trim()));
        anyhow::bail!("tmux command failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        log::debug(&format!("tmux {} -> {}", args.join(" "), stdout.trim()));
    }

    Ok(output)
}

/// Execute a tmux command interactively (for attach)
fn execute_tmux_interactive(args: &[&str]) -> Result<()> {
    log::debug(&format!("tmux {}", args.join(" ")));

    let status = Command::new("tmux")
        .args(args)
        .status()
        .context("Failed to execute tmux command")?;

    if !status.success() {
        log::error(&format!("tmux {} -> exit status: {}", args.join(" "), status));
        anyhow::bail!("tmux command failed with status: {}", status);
    }

    Ok(())
}
