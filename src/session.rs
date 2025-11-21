use crate::config::Session;
use crate::tmux;
use anyhow::Result;

pub fn create_session(session: &Session) -> Result<()> {
    // Validate session
    session.validate()?;

    // Get tmux base-index
    let base_index = tmux::get_base_index()?;

    let session_name = &session.name;
    let root = session.root_expanded();
    let root_opt = if root.is_empty() || root == "~" {
        None
    } else {
        Some(root.as_str())
    };

    println!("Creating session '{}' with {} window(s)...", session_name, session.windows.len());

    // Create the session with the first window
    let first_window_name = &session.windows[0].name;
    tmux::new_session(session_name, first_window_name, root_opt)?;

    // Process each window
    for (i, window) in session.windows.iter().enumerate() {
        let window_index = base_index + i;

        // Create window (first window already exists)
        if i > 0 {
            tmux::new_window(session_name, &window.name, root_opt)?;
        }

        // Create panes for this window
        let pane_count = window.panes.len();

        // If more than one pane, create additional panes
        if pane_count > 1 {
            for j in 1..pane_count {
                // Alternate between horizontal and vertical splits
                let horizontal = j % 2 == 1;
                tmux::split_window(session_name, window_index, horizontal, root_opt)?;
            }

            // Apply layout after creating all panes
            let layout = if pane_count == 2 {
                "even-horizontal"
            } else {
                "tiled"
            };
            tmux::select_layout(session_name, window_index, layout)?;
        }

        // Send commands to all panes in this window
        for (j, pane) in window.panes.iter().enumerate() {
            // Send environment variables first
            for (key, value) in &pane.env {
                let export_cmd = format!("export {}={}", key, shell_escape(value));
                tmux::send_keys(session_name, window_index, j, &export_cmd)?;
            }

            // Send the command
            if !pane.command.is_empty() {
                tmux::send_keys(session_name, window_index, j, &pane.command)?;
            }
        }
    }

    // Select the first window and pane
    tmux::select_window(session_name, base_index)?;
    tmux::select_pane(session_name, base_index, 0)?;

    println!("✓ Session '{}' created", session_name);
    println!("  Windows: {}", session.windows.len());

    // Show summary
    for window in &session.windows {
        println!("    - {}: {} pane(s)", window.name, window.panes.len());
    }

    Ok(())
}

/// Simple shell escaping for environment variable values
fn shell_escape(s: &str) -> String {
    if s.contains(|c: char| c.is_whitespace() || "'\"`$\\".contains(c)) {
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
}
