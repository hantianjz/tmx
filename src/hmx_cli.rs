use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "hmx",
    version,
    about = "A Herdr workspace manager using tmx configuration"
)]
pub struct Cli {
    /// Path to config file (default: ~/.config/tmx/tmx.toml)
    #[arg(short, long, global = true)]
    pub config: Option<String>,

    /// Manage workspaces on this SSH target
    #[arg(long, global = true)]
    pub remote: Option<String>,

    /// Select a named Herdr session
    #[arg(long, global = true)]
    pub session: Option<String>,

    /// Print Herdr and SSH commands
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Open or focus a workspace
    #[command(alias = "o")]
    Open { workspace: String },
    /// Close a running workspace
    #[command(alias = "c")]
    Close { workspace: String },
    /// Add missing configured tabs and panes
    #[command(alias = "r")]
    Refresh { workspace: String },
    /// List configured and running workspaces
    #[command(alias = "ls")]
    List,
    /// Initialize the shared tmx configuration file
    Init,
    /// Validate the shared configuration
    Validate,
    /// Manage configured Herdr machines
    Machines {
        #[command(subcommand)]
        command: MachineCommands,
    },
    /// Generate shell completions
    Completions { shell: String },
    #[command(name = "__list-configured", hide = true)]
    ListConfigured,
    #[command(name = "__list-running", hide = true)]
    ListRunning,
}

#[derive(Debug, Subcommand)]
pub enum MachineCommands {
    /// Register configured remote sessions alongside Local
    Sync,
}
