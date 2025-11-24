use crate::context::Context as AppContext;
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
    // Get config from context (lazy-loaded)
    let config = ctx.config()?;

    // Find session in config
    let session = config
        .get_session(session_id)
        .ok_or_else(|| anyhow::anyhow!("Session '{}' not found in configuration", session_id))?;

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
