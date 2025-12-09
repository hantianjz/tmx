use crate::shells;
use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub enum Shell {
    Fish,
    Bash,
    Zsh,
}

impl std::str::FromStr for Shell {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "fish" => Ok(Shell::Fish),
            "bash" => Ok(Shell::Bash),
            "zsh" => Ok(Shell::Zsh),
            _ => anyhow::bail!("Unsupported shell: {}. Supported shells: fish, bash, zsh", s),
        }
    }
}

pub fn run_completions(shell: Shell) -> Result<()> {
    match shell {
        Shell::Fish => {
            println!("{}", shells::fish::generate_completions());
        }
        Shell::Bash => {
            println!("{}", shells::bash::generate_completions());
        }
        Shell::Zsh => {
            println!("{}", shells::zsh::generate_completions());
        }
    }
    Ok(())
}
