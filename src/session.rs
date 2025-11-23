use crate::config::Session;
use crate::tmux;
use anyhow::Result;

/// Create a new tmux session from a configuration.
///
/// This function validates the session, creates all windows and panes,
/// applies layouts, sends commands, and selects the startup window/pane.
///
/// # Arguments
/// * `session` - The session configuration to create
///
/// # Errors
/// Returns an error if validation fails, tmux commands fail, or if
/// any part of the session creation process encounters an issue.
pub fn create_session(session: &Session) -> Result<()> {
    // Validate session
    session.validate()?;

    // Get tmux base-index
    let base_index = tmux::get_base_index()?;

    let session_name = &session.name;
    let session_root = session.root_expanded();

    println!(
        "Creating session '{}' with {} window(s)...",
        session_name,
        session.windows.len()
    );

    // Create the session with the first window
    let first_window_name = &session.windows[0].name;
    let first_window_root = session.windows[0].root_expanded(&session_root);
    tmux::new_session(session_name, first_window_name, Some(&first_window_root))?;

    // Process each window
    for (window_offset, window) in session.windows.iter().enumerate() {
        let window_index = base_index + window_offset;
        let window_root = window.root_expanded(&session_root);

        // Create window (first window already exists)
        if window_offset > 0 {
            tmux::new_window(session_name, &window.name, Some(&window_root))?;
        }

        // Create panes for this window
        let pane_count = window.panes.len();

        if pane_count > 1 {
            // Create additional panes (first pane already exists)
            for pane_idx in 1..pane_count {
                let pane = &window.panes[pane_idx];
                let pane_root = pane.root_expanded(&window_root);

                // Determine split direction
                let horizontal = determine_split_direction(pane_idx, pane);

                // Create the pane with optional size
                tmux::split_window_with_size(
                    session_name,
                    window_index,
                    horizontal,
                    pane.size.as_deref(),
                    Some(&pane_root),
                )?;
            }

            // Apply layout only if no custom sizes are specified
            // (select-layout overrides custom sizes, so we skip it if sizes are set)
            if should_apply_layout(window) {
                let layout = determine_layout(window, pane_count);
                tmux::select_layout(session_name, window_index, layout)?;
            }
        }

        // Send commands to all panes in this window
        for (pane_idx, pane) in window.panes.iter().enumerate() {
            // Note: Working directory is already set via -c flag when creating the pane
            // so we don't need to cd here

            // Send environment variables
            for (key, value) in &pane.env {
                let export_cmd = format!("export {}={}", key, shell_escape(value));
                tmux::send_keys(session_name, window_index, pane_idx, &export_cmd)?;
            }

            // Send the command
            if !pane.command.is_empty() {
                tmux::send_keys(session_name, window_index, pane_idx, &pane.command)?;
            }
        }
    }

    // Select the startup window and pane
    let startup_window_idx = base_index + session.resolve_startup_window();
    let startup_pane = session.get_startup_pane();

    tmux::select_window(session_name, startup_window_idx)?;
    tmux::select_pane(session_name, startup_window_idx, startup_pane)?;

    println!("✓ Session '{}' created", session_name);
    println!("  Windows: {}", session.windows.len());

    // Show summary
    for window in &session.windows {
        println!("    - {}: {} pane(s)", window.name, window.panes.len());
    }

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

/// Simple shell escaping for environment variable values
fn shell_escape(s: &str) -> String {
    const SPECIAL_CHARS: &str = "'\"`$\\";
    let needs_escaping = s
        .chars()
        .any(|c| c.is_whitespace() || SPECIAL_CHARS.contains(c));

    if needs_escaping {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("simple"), "simple");
        assert_eq!(shell_escape("with space"), "'with space'");
        assert_eq!(shell_escape("with'quote"), "'with'\\''quote'");
        assert_eq!(shell_escape("$VAR"), "'$VAR'");
    }

    #[test]
    fn test_determine_split_direction_explicit() {
        let pane = crate::config::Pane {
            command: String::new(),
            env: std::collections::HashMap::new(),
            root: None,
            split: Some("horizontal".to_string()),
            size: None,
        };
        assert!(determine_split_direction(0, &pane));

        let pane = crate::config::Pane {
            split: Some("vertical".to_string()),
            ..pane
        };
        assert!(!determine_split_direction(0, &pane));
    }

    #[test]
    fn test_determine_split_direction_default() {
        let pane = crate::config::Pane {
            command: String::new(),
            env: std::collections::HashMap::new(),
            root: None,
            split: None,
            size: None,
        };
        // Odd indices = horizontal
        assert!(determine_split_direction(1, &pane));
        assert!(determine_split_direction(3, &pane));
        // Even indices = vertical
        assert!(!determine_split_direction(2, &pane));
        assert!(!determine_split_direction(4, &pane));
    }
}
