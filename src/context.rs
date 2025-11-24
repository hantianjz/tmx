use anyhow::Result;
use once_cell::sync::OnceCell;
use std::path::PathBuf;

use crate::config::Config;
use crate::tmux;

/// Shared context for commands containing configuration and cached state.
///
/// The context is created once at startup and passed to all commands.
/// It handles lazy loading of configuration and caching of expensive operations.
pub struct Context {
    /// Lazy-loaded configuration
    config: OnceCell<Config>,
    /// Path to config file (resolved from CLI arg > env var > default)
    config_path: PathBuf,
    /// Whether to print debug/verbose output (from -v flag)
    verbose: bool,
    /// Whether we're running inside a tmux session (checked once at startup)
    pub is_inside_tmux: bool,
    /// Cached tmux base-index (lazy-loaded)
    base_index: OnceCell<usize>,
}

impl Context {
    /// Create a new context with the given CLI arguments.
    ///
    /// This reads all environment variables at startup:
    /// - TMX_CONFIG_PATH: Custom config path
    /// - TMUX: Whether we're inside tmux
    ///
    /// # Arguments
    /// * `config_path` - Optional config path from CLI --config flag
    /// * `verbose` - Whether to enable verbose/debug output (from -v flag)
    pub fn new(config_path: Option<String>, verbose: bool) -> Result<Self> {
        // Resolve config path from: CLI arg > TMX_CONFIG_PATH env > default
        let resolved_path = if let Some(path) = config_path {
            PathBuf::from(shellexpand::tilde(&path).to_string())
        } else if let Ok(env_path) = std::env::var("TMX_CONFIG_PATH") {
            PathBuf::from(shellexpand::tilde(&env_path).to_string())
        } else {
            // Default path: ~/.config/tmx/tmx.toml
            Config::config_path()?
        };

        // Check if we're inside tmux (read TMUX env var once)
        let is_inside_tmux = std::env::var("TMUX").is_ok();

        Ok(Self {
            config: OnceCell::new(),
            config_path: resolved_path,
            verbose,
            is_inside_tmux,
            base_index: OnceCell::new(),
        })
    }

    /// Get the configuration, loading it lazily on first access.
    ///
    /// # Returns
    /// A reference to the loaded configuration.
    ///
    /// # Errors
    /// Returns an error if the config file cannot be read or parsed.
    pub fn config(&self) -> Result<&Config> {
        self.config
            .get_or_try_init(|| Config::load_from(&self.config_path))
    }

    /// Get the tmux base-index, caching it after the first query.
    ///
    /// # Returns
    /// The base-index value (typically 0 or 1).
    pub fn base_index(&self) -> Result<usize> {
        self.base_index
            .get_or_try_init(tmux::get_base_index)
            .copied()
    }

    /// Check if verbose/debug mode is enabled.
    ///
    /// When verbose mode is enabled, tmux commands should be printed.
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Get the config path (useful for displaying to user).
    #[allow(dead_code)]
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }
}
