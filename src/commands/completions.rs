use crate::shells;
use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub enum Shell {
    Fish,
}

impl std::str::FromStr for Shell {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "fish" => Ok(Shell::Fish),
            _ => anyhow::bail!("Unsupported shell: {}", s),
        }
    }
}

pub fn run_completions(shell: Shell) -> Result<()> {
    match shell {
        Shell::Fish => {
            println!("{}", shells::fish::generate_completions());
        }
    }
    Ok(())
}
