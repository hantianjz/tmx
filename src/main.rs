mod cli;
mod commands;
mod config;
mod session;
mod shells;
mod tmux;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Set custom config path if provided
    if let Some(ref config_path) = cli.config {
        std::env::set_var("TMX_CONFIG_PATH", config_path);
    }

    match cli.command {
        Some(Commands::Start { session }) => commands::start::run(&session),
        Some(Commands::Stop { session }) => commands::stop::run(&session),
        Some(Commands::List) => commands::list::run(),
        Some(Commands::Init) => commands::init::run(),
        Some(Commands::Validate) => commands::validate::run(),
        Some(Commands::Completions { shell }) => {
            let shell = shell.parse()?;
            commands::completions::run_completions(shell)
        }
        Some(Commands::ListConfigured) => commands::list::list_configured(),
        Some(Commands::ListRunning) => commands::list::list_running(),
        None => {
            // Default command: list sessions
            commands::list::run()
        }
    }
}
