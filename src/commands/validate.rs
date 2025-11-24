use crate::context::Context;
use anyhow::Result;

pub fn run(ctx: &Context) -> Result<()> {
    // Get config from context (lazy-loaded)
    let config = ctx.config()?;

    let mut has_warnings = false;

    // Validate each session
    for (id, session) in &config.sessions {
        if let Err(e) = session.validate() {
            eprintln!("✗ Validation failed for session '{}':\n", id);
            eprintln!("{}", e);
            std::process::exit(1);
        }

        // Check for warnings: layout specified with custom pane sizes
        for window in &session.windows {
            if window.layout.is_some() && window.panes.iter().any(|p| p.size.is_some()) {
                if !has_warnings {
                    println!();
                    println!("⚠ Warnings:");
                    has_warnings = true;
                }
                println!(
                    "  Session '{}', window '{}': both layout and pane sizes specified - sizes will override layout",
                    id, window.name
                );
            }
        }
    }

    if has_warnings {
        println!();
    }

    println!("✓ Configuration is valid");
    println!("  Found {} session(s)", config.sessions.len());

    Ok(())
}
