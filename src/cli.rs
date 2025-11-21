use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tmx")]
#[command(version, about = "A tmux session manager with declarative TOML configuration", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start or attach to a session
    Start {
        /// Session name or ID from config
        session: String,
    },

    /// Stop a running session
    Stop {
        /// Session name to stop
        session: String,
    },

    /// List configured and running sessions (default)
    #[command(alias = "ls")]
    List,

    /// Show only running tmux sessions
    Running,

    /// Initialize configuration file
    Init,

    /// Validate configuration syntax
    Validate,

    /// Generate shell completions
    Completions {
        /// Shell type (fish, bash, zsh)
        shell: String,
    },

    /// Generate shell alias setup
    Alias {
        /// Shell type (fish, bash, zsh)
        shell: String,
    },

    /// Complete shell integration setup
    Setup {
        /// Shell type (fish, bash, zsh)
        shell: String,
    },

    /// List configured sessions (hidden, for completions)
    #[command(name = "__list-configured", hide = true)]
    ListConfigured,

    /// List running sessions (hidden, for completions)
    #[command(name = "__list-running", hide = true)]
    ListRunning,
}
