use crate::config::Config;
use anyhow::Result;

pub fn run() -> Result<()> {
    let config_path = Config::config_path()?;

    if !config_path.exists() {
        anyhow::bail!(
            "Config file not found at {}\nRun 'tmx init' to create one.",
            config_path.display()
        );
    }

    // Try to load the config
    let config = Config::load()?;

    // Validate each session
    for (id, session) in &config.sessions {
        session.validate().map_err(|e| {
            anyhow::anyhow!("Validation failed for session '{}': {}", id, e)
        })?;
    }

    println!("✓ Configuration is valid");
    println!("  Found {} session(s)", config.sessions.len());

    Ok(())
}
