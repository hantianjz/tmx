use crate::config::{Config, DEFAULT_CONFIG};
use anyhow::{Context, Result};
use std::fs;

pub fn run() -> Result<()> {
    let config_path = Config::config_path()?;
    let config_dir = Config::config_dir()?;

    // Check if config already exists
    if config_path.exists() {
        println!(
            "Configuration file already exists at {}",
            config_path.display()
        );
        println!("Edit it with: $EDITOR {}", config_path.display());
        return Ok(());
    }

    // Create config directory if it doesn't exist
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                config_dir.display()
            )
        })?;
    }

    // Write default configuration
    fs::write(&config_path, DEFAULT_CONFIG)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    println!("✓ Configuration file created at {}", config_path.display());
    println!();
    println!("Edit it with: $EDITOR {}", config_path.display());
    println!("Then start a session with: tmx start dev");

    Ok(())
}
