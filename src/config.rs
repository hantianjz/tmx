use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub sessions: HashMap<String, Session>,
}

/// Session configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Session {
    pub name: String,
    #[serde(default = "default_root")]
    pub root: String,
    pub windows: Vec<Window>,
}

/// Window configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Window {
    pub name: String,
    pub panes: Vec<Pane>,
}

/// Pane configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Pane {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

fn default_root() -> String {
    "~".to_string()
}

impl Config {
    /// Load configuration from the default location
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        Self::load_from(&path)
    }

    /// Load configuration from a specific path
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        // Validate that there's at least one session
        if config.sessions.is_empty() {
            anyhow::bail!("Config file contains no sessions");
        }

        Ok(config)
    }

    /// Get the default config file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("tmx");

        Ok(config_dir.join("tmx.toml"))
    }

    /// Get the config directory
    pub fn config_dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("tmx");

        Ok(config_dir)
    }

    /// Get a session by name or ID
    pub fn get_session(&self, name: &str) -> Option<&Session> {
        // Try direct lookup first
        if let Some(session) = self.sessions.get(name) {
            return Some(session);
        }

        // Try finding by session name field
        self.sessions.values().find(|s| s.name == name)
    }

    /// List all session names (from TOML keys)
    pub fn session_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.sessions.keys().cloned().collect();
        ids.sort();
        ids
    }
}

impl Session {
    /// Get the expanded root directory
    pub fn root_expanded(&self) -> String {
        shellexpand::tilde(&self.root).to_string()
    }

    /// Validate the session configuration
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            anyhow::bail!("Session name cannot be empty");
        }

        if self.windows.is_empty() {
            anyhow::bail!("Session '{}' must have at least one window", self.name);
        }

        for (i, window) in self.windows.iter().enumerate() {
            window
                .validate()
                .with_context(|| format!("Window {} in session '{}'", i, self.name))?;
        }

        Ok(())
    }
}

impl Window {
    /// Validate the window configuration
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            anyhow::bail!("Window name cannot be empty");
        }

        if self.panes.is_empty() {
            anyhow::bail!("Window '{}' must have at least one pane", self.name);
        }

        Ok(())
    }
}

/// Default configuration template
pub const DEFAULT_CONFIG: &str = r#"# TMX Configuration
# Define your tmux sessions here

# Example development session
[sessions.dev]
name = "dev"
root = "~/projects"
windows = [
    { name = "editor", panes = [
        { command = "nvim" }
    ]},
    { name = "terminal", panes = [
        { command = "" }
    ]}
]

# Example session with multiple panes
[sessions.work]
name = "work"
root = "~/work"
windows = [
    { name = "code", panes = [
        { command = "nvim" }
    ]},
    { name = "servers", panes = [
        { command = "echo 'Backend server'", env = { NODE_ENV = "development" } },
        { command = "echo 'Frontend server'" }
    ]},
    { name = "shell", panes = [
        { command = "" }
    ]}
]
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_config() {
        let config: Config = toml::from_str(DEFAULT_CONFIG).expect("Failed to parse default config");
        assert_eq!(config.sessions.len(), 2);
        assert!(config.sessions.contains_key("dev"));
        assert!(config.sessions.contains_key("work"));
    }

    #[test]
    fn test_session_validation() {
        let config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
        for session in config.sessions.values() {
            session.validate().expect("Session validation failed");
        }
    }

    #[test]
    fn test_root_expansion() {
        let session = Session {
            name: "test".to_string(),
            root: "~/projects".to_string(),
            windows: vec![],
        };
        let expanded = session.root_expanded();
        assert!(!expanded.contains('~'));
    }
}
