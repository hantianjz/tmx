mod cli;
mod commands;
mod config;
mod context;
mod log;
mod session;
mod shells;
mod tmux;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use context::Context;

fn main() {
    // Parse CLI first to get verbose flag
    let cli = Cli::parse();

    // Initialize logging to ~/.cache/tmx/tmx.log
    // Pass verbose flag to enable debug level logging
    log::init(cli.verbose);

    if let Err(e) = run(cli) {
        log::error(&format!("{}", e));
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {

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
            commands::default::run(&ctx)
        }
    }
}
