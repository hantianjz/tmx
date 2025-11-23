use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub sessions: HashMap<String, Session>,
    #[serde(default)]
    pub default: Option<String>,
}

/// Startup window specification (by name or index)
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum StartupWindow {
    Name(String),
    Index(usize),
}

/// Session configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Session {
    pub name: String,
    #[serde(default = "default_root")]
    pub root: String,
    pub windows: Vec<Window>,
    #[serde(default)]
    pub startup_window: Option<StartupWindow>,
    #[serde(default)]
    pub startup_pane: Option<usize>,
}

/// Window configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Window {
    pub name: String,
    pub panes: Vec<Pane>,
    #[serde(default)]
    pub layout: Option<String>,
    #[serde(default)]
    pub root: Option<String>,
}

/// Pane configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Pane {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub split: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
}

fn default_root() -> String {
    "~".to_string()
}

/// Helper for creating startup window index validation errors
fn startup_window_index_error(
    session_name: &str,
    found: usize,
    max: usize,
    count: usize,
) -> anyhow::Error {
    anyhow::anyhow!(
        "Invalid startup_window index in session '{}'\n  \
         Found: {}\n  \
         Valid range: 0 to {} (session has {} window(s))\n  \
         Hint: Window indices are 0-based",
        session_name,
        found,
        max,
        count
    )
}

/// Helper for creating startup window name validation errors
fn startup_window_name_error(session_name: &str, found: &str, available: &[&str]) -> anyhow::Error {
    anyhow::anyhow!(
        "Invalid startup_window value in session '{}'\n  \
         Found: '{}'\n  \
         Available windows:\n    \
         - {}\n  \
         Hint: Use a window name from your windows list or a 0-based index",
        session_name,
        found,
        available.join("\n    - ")
    )
}

/// Helper for creating layout validation errors
fn invalid_layout_error(window_name: &str, found: &str, valid_layouts: &[&str]) -> anyhow::Error {
    anyhow::anyhow!(
        "Invalid layout value in window '{}'\n  \
         Found: '{}'\n  \
         Valid layouts are:\n    \
         - {}\n  \
         Hint: Use 'even-horizontal' for side-by-side panes or 'tiled' for grid layout",
        window_name,
        found,
        valid_layouts.join("\n    - ")
    )
}

/// Helper for creating split direction validation errors
fn invalid_split_error(pane_index: usize, window_name: &str, found: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "Invalid split value in pane {} of window '{}'\n  \
         Found: '{}'\n  \
         Valid values are:\n    \
         - horizontal (side-by-side split)\n    \
         - vertical (top-bottom split)",
        pane_index,
        window_name,
        found
    )
}

impl Config {
    /// Load configuration from the default location.
    ///
    /// Uses the path from `TMX_CONFIG_PATH` environment variable if set,
    /// otherwise defaults to `~/.config/tmx/tmx.toml`.
    ///
    /// # Errors
    /// Returns an error if the config file cannot be read, parsed, or is empty.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        Self::load_from(&path)
    }

    /// Load configuration from a specific path.
    ///
    /// # Arguments
    /// * `path` - Path to the TOML configuration file
    ///
    /// # Errors
    /// Returns an error if the file cannot be read, is invalid TOML,
    /// or contains no sessions.
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

    /// Get the config file path (respects TMX_CONFIG_PATH env var)
    pub fn config_path() -> Result<PathBuf> {
        // Check for custom config path from environment variable
        if let Ok(custom_path) = std::env::var("TMX_CONFIG_PATH") {
            return Ok(PathBuf::from(shellexpand::tilde(&custom_path).to_string()));
        }

        // Use default path
        let config_dir = Self::config_dir()?;
        Ok(config_dir.join("tmx.toml"))
    }

    /// Get the config directory (always ~/.config/tmx)
    pub fn config_dir() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;

        Ok(home_dir.join(".config").join("tmx"))
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
        ids.sort_unstable();
        ids
    }
}

impl Session {
    /// Get the expanded root directory
    pub fn root_expanded(&self) -> String {
        shellexpand::tilde(&self.root).to_string()
    }

    /// Resolve startup window to index
    pub fn resolve_startup_window(&self) -> usize {
        let max_index = self.windows.len().saturating_sub(1);
        match &self.startup_window {
            Some(StartupWindow::Index(i)) => (*i).min(max_index),
            Some(StartupWindow::Name(name)) => self
                .windows
                .iter()
                .position(|w| &w.name == name)
                .unwrap_or(0),
            None => 0,
        }
    }

    /// Get the startup pane index (default 0)
    pub fn get_startup_pane(&self) -> usize {
        self.startup_pane.unwrap_or(0)
    }

    /// Validate the session configuration
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            anyhow::bail!("Session name cannot be empty");
        }

        if self.windows.is_empty() {
            anyhow::bail!("Session '{}' must have at least one window", self.name);
        }

        // Validate startup_window if specified
        if let Some(StartupWindow::Index(i)) = &self.startup_window {
            if *i >= self.windows.len() {
                return Err(startup_window_index_error(
                    &self.name,
                    *i,
                    self.windows.len() - 1,
                    self.windows.len(),
                ));
            }
        }

        if let Some(StartupWindow::Name(name)) = &self.startup_window {
            if !self.windows.iter().any(|w| &w.name == name) {
                let available: Vec<_> = self.windows.iter().map(|w| w.name.as_str()).collect();
                return Err(startup_window_name_error(&self.name, name, &available));
            }
        }

        for (i, window) in self.windows.iter().enumerate() {
            window.validate().map_err(|e| {
                anyhow::anyhow!(
                    "Window {} ('{}') in session '{}':\n{}",
                    i,
                    window.name,
                    self.name,
                    e
                )
            })?;
        }

        Ok(())
    }
}

impl Window {
    /// Valid tmux layouts
    const VALID_LAYOUTS: &'static [&'static str] = &[
        "even-horizontal",
        "even-vertical",
        "main-horizontal",
        "main-vertical",
        "tiled",
    ];

    /// Validate the window configuration
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            anyhow::bail!("Window name cannot be empty");
        }

        if self.panes.is_empty() {
            anyhow::bail!("Window '{}' must have at least one pane", self.name);
        }

        // Validate layout if specified
        if let Some(ref layout) = self.layout {
            if !Self::VALID_LAYOUTS.contains(&layout.as_str()) {
                return Err(invalid_layout_error(
                    &self.name,
                    layout,
                    Self::VALID_LAYOUTS,
                ));
            }
        }

        // Validate pane split directions
        for (i, pane) in self.panes.iter().enumerate() {
            if let Some(ref split) = pane.split {
                if split != "horizontal" && split != "vertical" {
                    return Err(invalid_split_error(i, &self.name, split));
                }
            }

            // Validate pane size format if specified
            if let Some(ref size) = pane.size {
                validate_size_format(size, i, &self.name)?;
            }
        }

        Ok(())
    }

    /// Get the expanded root directory for this window
    pub fn root_expanded(&self, session_root: &str) -> String {
        if let Some(ref root) = self.root {
            shellexpand::tilde(root).to_string()
        } else {
            shellexpand::tilde(session_root).to_string()
        }
    }
}

impl Pane {
    /// Get the expanded root directory for this pane
    pub fn root_expanded(&self, window_root: &str) -> String {
        if let Some(ref root) = self.root {
            shellexpand::tilde(root).to_string()
        } else {
            window_root.to_string()
        }
    }
}

/// Validate pane size format
fn validate_size_format(size: &str, pane_index: usize, window_name: &str) -> Result<()> {
    let is_valid = if let Some(percent_str) = size.strip_suffix('%') {
        // Percentage: should be a number between 1-100
        percent_str
            .parse::<u32>()
            .map(|n| (1..=100).contains(&n))
            .unwrap_or(false)
    } else {
        // Absolute: should be a positive number
        size.parse::<u32>().map(|n| n > 0).unwrap_or(false)
    };

    if !is_valid {
        anyhow::bail!(
            "Invalid size value in pane {} of window '{}'\n  \
             Found: '{}'\n  \
             Valid formats:\n    \
             - Percentage: '30%', '50%' (between 1% and 100%)\n    \
             - Absolute: '20', '40' (positive number of lines or columns)\n  \
             Example: size = \"30%\" or size = \"20\"",
            pane_index,
            window_name,
            size
        );
    }

    Ok(())
}

/// Default configuration template
pub const DEFAULT_CONFIG: &str = r#"# TMX Configuration
# Define your tmux sessions here

# Default session to start when no sessions are running (optional)
default = "dev"

# Simple development session
[sessions.dev]
name = "dev"
root = "~/projects"
startup_window = 0              # Focus first window (0-based index)

[[sessions.dev.windows]]
name = "editor"
layout = "main-vertical"        # Large main pane, smaller side pane

[[sessions.dev.windows.panes]]
command = "nvim"                # Main pane

[[sessions.dev.windows.panes]]
command = ""
size = "25%"                    # Side pane takes 25% width

[[sessions.dev.windows]]
name = "terminal"

[[sessions.dev.windows.panes]]
command = ""

# Advanced session with multiple features
[sessions.work]
name = "work"
root = "~/work"
startup_window = "code"         # Can use window name instead of index

[[sessions.work.windows]]
name = "code"
layout = "even-horizontal"      # Panes evenly distributed horizontally

[[sessions.work.windows.panes]]
command = "nvim"

[[sessions.work.windows.panes]]
command = ""

[[sessions.work.windows]]
name = "servers"
layout = "tiled"                # Grid layout for multiple panes

[[sessions.work.windows.panes]]
command = "echo 'Backend server'"
env = { NODE_ENV = "development" }

[[sessions.work.windows.panes]]
command = "echo 'Frontend server'"
split = "horizontal"            # Explicitly horizontal split
size = "50%"                    # Takes 50% of space

[[sessions.work.windows.panes]]
command = "echo 'Database'"
root = "~/work/database"        # Per-pane working directory
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_config() {
        let config: Config =
            toml::from_str(DEFAULT_CONFIG).expect("Failed to parse default config");
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
            startup_window: None,
            startup_pane: None,
        };
        let expanded = session.root_expanded();
        assert!(!expanded.contains('~'));
    }

    #[test]
    fn test_startup_window_by_index() {
        let config: Config = toml::from_str(
            r#"
[sessions.test]
name = "test"
root = "~"
startup_window = 1

[[sessions.test.windows]]
name = "first"
[[sessions.test.windows.panes]]
command = ""

[[sessions.test.windows]]
name = "second"
[[sessions.test.windows.panes]]
command = ""
"#,
        )
        .unwrap();

        let session = config.sessions.get("test").unwrap();
        assert_eq!(session.resolve_startup_window(), 1);
    }

    #[test]
    fn test_startup_window_by_name() {
        let config: Config = toml::from_str(
            r#"
[sessions.test]
name = "test"
root = "~"
startup_window = "second"

[[sessions.test.windows]]
name = "first"
[[sessions.test.windows.panes]]
command = ""

[[sessions.test.windows]]
name = "second"
[[sessions.test.windows.panes]]
command = ""
"#,
        )
        .unwrap();

        let session = config.sessions.get("test").unwrap();
        assert_eq!(session.resolve_startup_window(), 1);
    }

    #[test]
    fn test_window_layout_validation() {
        let config: Config = toml::from_str(
            r#"
[sessions.test]
name = "test"

[[sessions.test.windows]]
name = "win"
layout = "main-vertical"

[[sessions.test.windows.panes]]
command = ""
"#,
        )
        .unwrap();

        let session = config.sessions.get("test").unwrap();
        assert!(session.validate().is_ok());
    }

    #[test]
    fn test_invalid_layout() {
        let config: Config = toml::from_str(
            r#"
[sessions.test]
name = "test"

[[sessions.test.windows]]
name = "win"
layout = "invalid-layout"

[[sessions.test.windows.panes]]
command = ""
"#,
        )
        .unwrap();

        let session = config.sessions.get("test").unwrap();
        assert!(session.validate().is_err());
    }

    #[test]
    fn test_pane_sizing() {
        let config: Config = toml::from_str(
            r#"
[sessions.test]
name = "test"

[[sessions.test.windows]]
name = "win"

[[sessions.test.windows.panes]]
command = "nvim"

[[sessions.test.windows.panes]]
command = ""
size = "30%"
"#,
        )
        .unwrap();

        let session = config.sessions.get("test").unwrap();
        assert_eq!(session.windows[0].panes[1].size.as_deref(), Some("30%"));
    }

    #[test]
    fn test_per_window_root() {
        let config: Config = toml::from_str(
            r#"
[sessions.test]
name = "test"
root = "~/projects"

[[sessions.test.windows]]
name = "win1"

[[sessions.test.windows.panes]]
command = ""

[[sessions.test.windows]]
name = "win2"
root = "~/other"

[[sessions.test.windows.panes]]
command = ""
"#,
        )
        .unwrap();

        let session = config.sessions.get("test").unwrap();
        assert_eq!(session.windows[1].root.as_deref(), Some("~/other"));
    }

    #[test]
    fn test_per_pane_root() {
        let config: Config = toml::from_str(
            r#"
[sessions.test]
name = "test"

[[sessions.test.windows]]
name = "win"

[[sessions.test.windows.panes]]
command = "npm start"
root = "~/service-a"

[[sessions.test.windows.panes]]
command = "npm start"
root = "~/service-b"
"#,
        )
        .unwrap();

        let session = config.sessions.get("test").unwrap();
        assert_eq!(
            session.windows[0].panes[0].root.as_deref(),
            Some("~/service-a")
        );
        assert_eq!(
            session.windows[0].panes[1].root.as_deref(),
            Some("~/service-b")
        );
    }

    #[test]
    fn test_split_direction() {
        let config: Config = toml::from_str(
            r#"
[sessions.test]
name = "test"

[[sessions.test.windows]]
name = "win"

[[sessions.test.windows.panes]]
command = ""

[[sessions.test.windows.panes]]
command = ""
split = "horizontal"
"#,
        )
        .unwrap();

        let session = config.sessions.get("test").unwrap();
        assert_eq!(
            session.windows[0].panes[1].split.as_deref(),
            Some("horizontal")
        );
    }
}
