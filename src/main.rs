mod cli;
mod commands;
mod config;
mod context;
mod session;
mod shells;
mod tmux;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use context::Context;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Create context once with all CLI arguments and env vars
    let ctx = Context::new(cli.config, cli.verbose)?;

    match cli.command {
        Some(Commands::Open { session }) => commands::start::run(&session, &ctx),
        Some(Commands::Close { session }) => commands::stop::run(&session, &ctx),
        Some(Commands::Refresh { session }) => commands::refresh::run(&session, &ctx),
        Some(Commands::List) => commands::list::run(&ctx),
        Some(Commands::Init) => commands::init::run(),
        Some(Commands::Validate) => commands::validate::run(&ctx),
        Some(Commands::Completions { shell }) => {
            let shell = shell.parse()?;
            commands::completions::run_completions(shell)
        }
        Some(Commands::ListConfigured) => commands::list::list_configured(&ctx),
        Some(Commands::ListRunning) => commands::list::list_running(),
        None => {
            // Default command: cycle through sessions
            commands::cycle::run(&ctx)
        }
    }
}
