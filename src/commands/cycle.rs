use crate::config::Config;
use crate::context::Context as AppContext;
use crate::tmux;
use anyhow::{Context, Result};

/// Cycle through running tmux sessions, or start the first configured session if none are running.
///
/// Behavior:
/// - If no sessions are running: start the first configured session
/// - If inside tmux: get current session and switch to next
/// - If outside tmux: attach to first running session
///
/// Cycling order: configured sessions (alphabetically), then unconfigured sessions (alphabetically)
pub fn run(ctx: &AppContext) -> Result<()> {
    // Check if tmux is installed
    if !tmux::is_installed() {
        anyhow::bail!("tmux is not installed");
    }

    // Get running sessions
    let running = tmux::list_sessions()?;

    if running.is_empty() {
        // No sessions running, start default or first from config
        let config = ctx.config().context("Failed to load configuration")?;

        // Use default session if specified, otherwise use first session
        let session_id = if let Some(ref default) = config.default {
            // Validate that the default session exists
            if !config.sessions.contains_key(default) {
                anyhow::bail!(
                    "Default session '{}' not found in configuration\nAvailable sessions: {}",
                    default,
                    config.session_ids().join(", ")
                );
            }
            default.clone()
        } else {
            // No default specified, use first session
            let session_ids = config.session_ids();
            if session_ids.is_empty() {
                anyhow::bail!("No sessions configured in tmx.toml");
            }
            session_ids[0].clone()
        };

        println!("No sessions running. Starting '{}'...", session_id);
        return crate::commands::start::run(&session_id, ctx);
    }

    // Get config from context to determine session ordering (only load once!)
    let config = ctx.config().ok();
    let ordered_sessions = order_sessions(&running, config);

    // If inside tmux, get current session and switch to next
    if ctx.is_inside_tmux {
        let current = tmux::get_current_session()?;
        let next = find_next_session(&ordered_sessions, &current);
        println!("Switching to session '{}'...", next);
        return tmux::switch_client(&next);
    }

    // Not in tmux, attach to first session
    let first = &ordered_sessions[0];
    println!("Attaching to session '{}'...", first);
    tmux::attach_session(first)
}

/// Order sessions: configured sessions first (alphabetically), then unconfigured sessions (alphabetically)
fn order_sessions(running: &[String], config: Option<&Config>) -> Vec<String> {
    let mut result = Vec::new();

    if let Some(cfg) = config {
        let configured_ids = cfg.session_ids();
        let configured_names: Vec<String> = configured_ids
            .iter()
            .filter_map(|id| {
                cfg.get_session(id)
                    .map(|s| s.name.clone())
                    .filter(|name| running.contains(name))
            })
            .collect();

        // Add configured sessions first (in alphabetical order of their IDs)
        for name in &configured_names {
            result.push(name.clone());
        }

        // Add unconfigured sessions (alphabetically)
        let mut unconfigured: Vec<String> = running
            .iter()
            .filter(|s| !configured_names.contains(s))
            .cloned()
            .collect();
        unconfigured.sort();
        result.extend(unconfigured);
    } else {
        // No config available, just use running sessions alphabetically
        let mut sorted = running.to_vec();
        sorted.sort();
        result = sorted;
    }

    result
}

/// Find the next session in the cycle
fn find_next_session(sessions: &[String], current: &str) -> String {
    let pos = sessions.iter().position(|s| s == current);
    match pos {
        Some(i) => {
            // Cycle to next session (wrap around to first if at end)
            sessions[(i + 1) % sessions.len()].clone()
        }
        None => {
            // Current session not in list, go to first
            sessions[0].clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_next_session() {
        let sessions = vec!["dev".to_string(), "work".to_string(), "test".to_string()];

        assert_eq!(find_next_session(&sessions, "dev"), "work");
        assert_eq!(find_next_session(&sessions, "work"), "test");
        assert_eq!(find_next_session(&sessions, "test"), "dev"); // wrap around

        // Current not in list
        assert_eq!(find_next_session(&sessions, "other"), "dev");
    }

    #[test]
    fn test_order_sessions_no_config() {
        let running = vec!["zebra".to_string(), "alpha".to_string(), "beta".to_string()];
        let ordered = order_sessions(&running, None);
        assert_eq!(ordered, vec!["alpha", "beta", "zebra"]);
    }
}
