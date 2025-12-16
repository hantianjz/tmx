use crate::context::Context as AppContext;
use crate::log;
use crate::session;
use crate::tmux;
use anyhow::{Context, Result};

/// Refresh the layout of a running session according to its configuration.
///
/// This function:
/// - Preserves running processes in existing panes
/// - Adds new panes if config has more panes than current session
/// - Keeps extra panes if current session has more panes than config
/// - Reapplies layout from configuration
///
/// # Arguments
/// * `session_id` - The session name or ID from config
/// * `ctx` - Shared context containing configuration and state
pub fn run(session_id: &str, ctx: &AppContext) -> Result<()> {
    log::info(&format!("refresh command: session_id={}", session_id));

    // Get config from context (lazy-loaded)
    let config = ctx.config()?;

    // Find session in config, or use default session's settings for unconfigured sessions
    let session = if let Some(s) = config.get_session(session_id) {
        log::info(&format!("found session '{}' in config", session_id));
        s.clone()
    } else {
        // Session not in config - use default session's settings with the requested name
        log::info(&format!("session '{}' not in config, using default layout", session_id));
        let default_id = config.default.as_ref().ok_or_else(|| {
            log::error(&format!("no default session configured for '{}'", session_id));
            anyhow::anyhow!(
                "Session '{}' not found and no default session configured",
                session_id
            )
        })?;

        let default_session = config.get_session(default_id).ok_or_else(|| {
            log::error(&format!("default session '{}' not found", default_id));
            anyhow::anyhow!(
                "Default session '{}' not found in configuration",
                default_id
            )
        })?;

        // Clone the default session and change the name
        let mut dynamic_session = default_session.clone();
        dynamic_session.name = session_id.to_string();
        // Use current working directory instead of the default session's root
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "~".to_string());
        dynamic_session.root = cwd.clone();
        log::info(&format!("using default session '{}' as template with root '{}'", default_id, cwd));
        dynamic_session
    };

    let session_name = &session.name;

    // Verify session exists in tmux
    if !tmux::has_session(session_name)? {
        anyhow::bail!("Session '{}' is not running", session_name);
    }

    println!("Refreshing layout for session '{}'...", session_name);

    // Get tmux base-index from context (cached)
    let base_index = ctx.base_index()?;
    let verbose = ctx.is_verbose();
    let session_root = session.root_expanded();

    // Process each window
    for (window_offset, window) in session.windows.iter().enumerate() {
        let window_index = base_index + window_offset;
        let window_root = window.root_expanded(&session_root);

        // Get current pane count
        let current_pane_count = tmux::count_panes(session_name, window_index)
            .context(format!("Failed to count panes in window {}", window_index))?;

        let expected_pane_count = window.panes.len();

        println!(
            "  Window '{}': current={} panes, config={} panes",
            window.name, current_pane_count, expected_pane_count
        );

        // Add new panes if config has more panes than current
        if current_pane_count < expected_pane_count {
            let panes_to_add = expected_pane_count - current_pane_count;
            println!("    Adding {} pane(s)...", panes_to_add);

            // Create additional panes using shared logic
            // Don't apply sizes here - let apply_window_layout handle it
            session::create_window_panes(
                session_name,
                window_index,
                window,
                &window_root,
                current_pane_count,
                false, // Don't apply sizes here - let apply_window_layout handle it
                verbose,
            )?;
        } else if current_pane_count > expected_pane_count {
            println!(
                "    Keeping {} extra pane(s) (not removing)",
                current_pane_count - expected_pane_count
            );
        }

        // Always apply layout and custom sizes during refresh
        if expected_pane_count > 1 {
            println!("    Applying layout and sizes...");
            session::apply_window_layout(session_name, window_index, window, verbose)?;
        }
    }

    println!("✓ Session '{}' layout refreshed", session_name);
    Ok(())
}
