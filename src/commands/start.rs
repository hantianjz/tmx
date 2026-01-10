use crate::context::Context;
use crate::log;
use crate::session;
use crate::tmux;
use anyhow::Result;

/// Attach to or switch to a tmux session depending on context.
///
/// If already inside tmux, switches the client to the target session.
/// Otherwise, attaches to the session from outside tmux.
fn attach_or_switch(session_name: &str, ctx: &Context) -> Result<()> {
    if ctx.is_inside_tmux {
        tmux::switch_client(session_name)
    } else {
        tmux::attach_session(session_name)
    }
}

/// Start or attach to a tmux session.
///
/// If the session already exists in tmux, we'll attach to it directly.
/// If not, we'll look it up in the configuration and create it.
///
/// # Arguments
/// * `session_id` - The session ID/name to attach to or create
/// * `ctx` - Shared context containing configuration and state
pub fn run(session_id: &str, ctx: &Context) -> Result<()> {
    log::info(&format!("open command: session_id={}", session_id));

    // Check if tmux is installed
    if !tmux::is_installed() {
        log::error("tmux is not installed");
        anyhow::bail!("tmux is not installed");
    }

    // First, check if a session with this name already exists in tmux
    // This allows attaching to any existing session, even if not in config
    if tmux::has_session(session_id)? {
        log::info(&format!("attaching to existing session '{}'", session_id));
        println!("Attaching to existing session '{}'...", session_id);
        return attach_or_switch(session_id, ctx);
    }

    // Session doesn't exist, so we need to create it from configuration
    let config = ctx.config()?;

    // Find the session in config, or use default session's layout for unconfigured sessions
    let (session, is_dynamic) = if let Some(s) = config.get_session(session_id) {
        log::info(&format!("found session '{}' in config", session_id));
        (s.clone(), false)
    } else {
        // Session not in config - use default session's layout with the requested name
        log::info(&format!("session '{}' not in config, using default layout", session_id));
        let default_id = config.default.as_ref().ok_or_else(|| {
            log::error(&format!("no default session configured for '{}'", session_id));
            anyhow::anyhow!(
                "Session '{}' not found and no default session configured\nAvailable sessions: {}",
                session_id,
                config.session_ids().join(", ")
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
        (dynamic_session, true)
    };

    let session_name = &session.name;
    let sanitized_name = tmux::sanitize_session_name(session_name);

    // Warn user if session name contains special characters
    if sanitized_name != *session_name {
        println!(
            "Note: Session name '{}' contains special characters and will be created as '{}'",
            session_name, sanitized_name
        );
    }

    // Double-check if session exists with the configured name (may differ from session_id)
    if tmux::has_session(session_name)? {
        println!("Attaching to existing session '{}'...", sanitized_name);
        attach_or_switch(session_name, ctx)?;
    } else {
        // Create the session
        if is_dynamic {
            println!("Creating session '{}' using default layout...", sanitized_name);
        }
        session::create_session(&session, ctx)?;
        // Attach to the newly created session
        attach_or_switch(session_name, ctx)?;
    }

    Ok(())
}
