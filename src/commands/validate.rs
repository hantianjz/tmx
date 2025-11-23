use crate::config::Config;
use anyhow::Result;

pub fn run(config: Config) -> Result<()> {
    // Validate each session
    for (id, session) in &config.sessions {
        if let Err(e) = session.validate() {
            eprintln!("✗ Validation failed for session '{}':\n", id);
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

    println!("✓ Configuration is valid");
    println!("  Found {} session(s)", config.sessions.len());

    Ok(())
}
