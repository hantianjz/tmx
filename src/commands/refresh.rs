use crate::config::Config;
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
pub fn run(session_id: &str) -> Result<()> {
    // Load configuration
    let config = Config::load().context("Failed to load configuration")?;

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

    // Get tmux base-index
    let base_index = tmux::get_base_index()?;
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

            for pane_idx in current_pane_count..expected_pane_count {
                let pane = &window.panes[pane_idx];
                let pane_root = pane.root_expanded(&window_root);

                // Determine split direction using the same logic as session creation
                let horizontal = determine_split_direction(pane_idx, pane);

                // Create the pane (without size, since we're refreshing)
                tmux::split_window(session_name, window_index, horizontal, Some(&pane_root))?;
            }
        } else if current_pane_count > expected_pane_count {
            println!(
                "    Keeping {} extra pane(s) (not removing)",
                current_pane_count - expected_pane_count
            );
        }

        // Reapply layout only if no custom sizes are specified
        if expected_pane_count > 1 && should_apply_layout(window) {
            let layout = determine_layout(window, expected_pane_count);
            println!("    Applying layout: {}", layout);
            tmux::select_layout(session_name, window_index, layout)?;
        }
    }

    println!("✓ Session '{}' layout refreshed", session_name);
    Ok(())
}

/// Check if we should apply a layout to the window
/// Returns false if any pane has a custom size (to preserve manual sizing)
fn should_apply_layout(window: &crate::config::Window) -> bool {
    // IMPORTANT: Tmux's select-layout command resets ALL pane sizes
    // So if ANY pane has a custom size, we must skip layout application
    // to preserve the user's sizing
    if window.panes.iter().any(|p| p.size.is_some()) {
        return false;
    }

    // Apply layout if explicitly set or use default for multi-pane windows
    true
}

/// Determine split direction based on pane config or default pattern
///
/// Returns `true` for horizontal split (side-by-side), `false` for vertical split (top-bottom).
/// If no explicit split direction is configured, uses an alternating pattern:
/// - Pane 1, 3, 5... → horizontal (side-by-side)
/// - Pane 2, 4, 6... → vertical (top-bottom)
fn determine_split_direction(pane_index: usize, pane: &crate::config::Pane) -> bool {
    if let Some(ref split) = pane.split {
        split == "horizontal"
    } else {
        // Default alternating pattern: odd indices get horizontal splits
        pane_index % 2 == 1
    }
}

/// Determine layout for window
fn determine_layout(window: &crate::config::Window, pane_count: usize) -> &str {
    if let Some(ref layout) = window.layout {
        layout
    } else {
        // Default behavior: even-horizontal for 2, tiled for 3+
        if pane_count == 2 {
            "even-horizontal"
        } else {
            "tiled"
        }
    }
}
