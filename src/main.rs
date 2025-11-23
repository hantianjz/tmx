mod cli;
mod commands;
mod config;
mod session;
mod shells;
mod tmux;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use config::Config;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Open { session }) => commands::start::run(&session, Config::load()?),
        Some(Commands::Close { session }) => commands::stop::run(&session),
        Some(Commands::Refresh { session }) => commands::refresh::run(&session, Config::load()?),
        Some(Commands::List) => commands::list::run(Config::load()?),
        Some(Commands::Init) => commands::init::run(),
        Some(Commands::Validate) => commands::validate::run(Config::load()?),
        Some(Commands::Completions { shell }) => {
            let shell = shell.parse()?;
            commands::completions::run_completions(shell)
        }
        Some(Commands::ListConfigured) => commands::list::list_configured(Config::load()?),
        Some(Commands::ListRunning) => commands::list::list_running(),
        None => {
            // Default command: cycle through sessions
            commands::cycle::run()
        }
    }
}
