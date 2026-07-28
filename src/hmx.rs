#[allow(dead_code)]
mod config;
mod herdr;
mod hmx_cli;
mod hmx_shells;
#[allow(dead_code)]
mod log;

use anyhow::{Context, Result, bail};
use clap::Parser;
use config::{Config, DEFAULT_CONFIG, Session};
use herdr::{Client, Runner, SystemRunner, Workspace, configured_order, next_workspace};
use hmx_cli::{Cli, Commands};
use std::collections::HashSet;
use std::path::PathBuf;

fn main() {
    let cli = Cli::parse();
    log::init_for("hmx", cli.verbose);
    if let Err(error) = run(cli) {
        log::error(&error.to_string());
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn config_path(explicit: Option<String>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(PathBuf::from(shellexpand::tilde(&path).to_string()));
    }
    for variable in ["HMX_CONFIG_PATH", "TMX_CONFIG_PATH"] {
        if let Ok(path) = std::env::var(variable) {
            return Ok(PathBuf::from(shellexpand::tilde(&path).to_string()));
        }
    }
    Config::config_path()
}

fn load_config(path: Option<String>) -> Result<Config> {
    Config::load_from(&config_path(path)?)
}

fn selected<'a>(config: &'a Config, name: &str) -> Result<&'a Session> {
    config.get_session(name).with_context(|| {
        format!(
            "Workspace '{}' is not configured\nAvailable workspaces: {}",
            name,
            config.session_ids().join(", ")
        )
    })
}

fn default_session(config: &Config) -> Result<&Session> {
    if let Some(default) = &config.default {
        return selected(config, default);
    }
    let id = config
        .session_ids()
        .into_iter()
        .next()
        .context("No workspaces configured")?;
    selected(config, &id)
}

fn dynamic_session<R: Runner>(client: &Client<R>, config: &Config, name: &str) -> Result<Session> {
    let default = config.default.as_ref().with_context(|| {
        format!(
            "Workspace '{}' is not configured and no default workspace is configured\nAvailable workspaces: {}",
            name,
            config.session_ids().join(", ")
        )
    })?;
    let mut session = selected(config, default)?.clone();
    session.name = name.to_string();
    session.root = if client.is_remote() {
        "~".to_string()
    } else {
        std::env::current_dir()
            .context("Could not determine current directory")?
            .to_string_lossy()
            .to_string()
    };
    Ok(session)
}

fn exact_workspace(running: &[Workspace], label: &str) -> Result<Option<Workspace>> {
    let matches: Vec<_> = running
        .iter()
        .filter(|workspace| workspace.label == label)
        .cloned()
        .collect();
    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.into_iter().next()),
        count => bail!(
            "Workspace label '{}' is ambiguous ({} running workspaces use it)",
            label,
            count
        ),
    }
}

fn unique_running_labels(running: Vec<Workspace>) -> Vec<String> {
    let mut seen = HashSet::new();
    running
        .into_iter()
        .filter_map(|workspace| {
            seen.insert(workspace.label.clone())
                .then_some(workspace.label)
        })
        .collect()
}

fn list_lines(config: &Config, running: &[Workspace]) -> Vec<String> {
    let mut lines = Vec::new();
    if running.is_empty() {
        lines.push("Configured workspaces:".to_string());
        lines.extend(config.session_ids().into_iter().map(|id| format!("  {id}")));
        lines.push(String::new());
    }
    lines.push("Running Herdr workspaces:".to_string());
    if running.is_empty() {
        lines.push("  (none)".to_string());
        return lines;
    }

    let mut represented_labels = HashSet::new();
    for id in config.session_ids() {
        let session = &config.sessions[&id];
        if running
            .iter()
            .any(|workspace| workspace.label == session.name)
        {
            lines.push(format!("  {id} (c)"));
            represented_labels.insert(session.name.clone());
        }
    }
    for label in unique_running_labels(running.to_vec()) {
        if !represented_labels.contains(&label) {
            lines.push(format!("  {label}"));
        }
    }
    lines
}

fn validation_warnings(config: &Config) -> Vec<String> {
    let mut warnings = Vec::new();
    for (id, session) in &config.sessions {
        for window in &session.windows {
            if window.layout.is_some() && window.panes.iter().any(|pane| pane.size.is_some()) {
                warnings.push(format!(
                    "  Workspace '{}', tab '{}': both layout and pane sizes specified - sizes will override layout",
                    id, window.name
                ));
            }
        }
    }
    warnings
}

fn run(cli: Cli) -> Result<()> {
    let inside = std::env::var_os("HERDR_ENV").is_some();
    if inside && cli.remote.is_some() {
        bail!("Cannot use --remote from inside Herdr; detach before targeting another server");
    }
    let client = Client::new(
        SystemRunner,
        cli.remote.clone(),
        cli.session.clone(),
        cli.verbose,
    );

    run_with_client(cli, &client, inside)
}

fn run_with_client<R: Runner>(cli: Cli, client: &Client<R>, inside: bool) -> Result<()> {
    match cli.command {
        Some(Commands::Init) => init_config(cli.config),
        Some(Commands::Validate) => {
            let config = load_config(cli.config)?;
            for session in config.sessions.values() {
                session.validate()?;
            }
            let warnings = validation_warnings(&config);
            if !warnings.is_empty() {
                println!("\n⚠ Warnings:");
                for warning in warnings {
                    println!("{warning}");
                }
                println!();
            }
            println!("✓ Shared configuration is valid");
            println!("  Found {} workspace(s)", config.sessions.len());
            Ok(())
        }
        Some(Commands::Completions { shell }) => {
            print!("{}", hmx_shells::generate(&shell)?);
            Ok(())
        }
        Some(Commands::ListConfigured) => {
            for id in load_config(cli.config)?.session_ids() {
                println!("{id}");
            }
            Ok(())
        }
        Some(Commands::ListRunning) => {
            for label in unique_running_labels(client.workspaces().unwrap_or_default()) {
                println!("{label}");
            }
            Ok(())
        }
        Some(Commands::List) => {
            let config = load_config(cli.config)?;
            let running = client.workspaces().unwrap_or_default();
            for line in list_lines(&config, &running) {
                println!("{line}");
            }
            Ok(())
        }
        Some(Commands::Open { workspace }) => {
            if let Ok(running) = client.workspaces()
                && let Some(found) = exact_workspace(&running, &workspace)?
            {
                client.focus_workspace(&found.id)?;
                println!("✓ Workspace '{}' is ready", found.label);
                return if inside { Ok(()) } else { client.attach() };
            }

            let config = load_config(cli.config)?;
            let session = config
                .get_session(&workspace)
                .cloned()
                .map(Ok)
                .unwrap_or_else(|| dynamic_session(client, &config, &workspace))?;
            client.ensure_server()?;

            // Resolve configured aliases, and close the race between the initial lookup and create.
            let running = client.workspaces()?;
            let found = exact_workspace(&running, &session.name)?;
            let ready = if let Some(found) = found {
                client.focus_workspace(&found.id)?;
                found
            } else {
                println!("Creating workspace '{}'...", session.name);
                client.create_workspace(&session)?
            };
            println!("✓ Workspace '{}' is ready", ready.label);
            if inside { Ok(()) } else { client.attach() }
        }
        Some(Commands::Close { workspace }) => {
            let all_running = client.workspaces()?;
            let (label, running) = if let Some(found) = exact_workspace(&all_running, &workspace)? {
                (workspace, found)
            } else {
                let config = load_config(cli.config)?;
                let label = config
                    .get_session(&workspace)
                    .map(|session| session.name.clone())
                    .unwrap_or(workspace);
                let found = exact_workspace(&all_running, &label)?
                    .with_context(|| format!("Workspace '{}' is not running", label))?;
                (label, found)
            };
            client.close_workspace(&running.id)?;
            println!("✓ Workspace '{}' closed", label);
            Ok(())
        }
        Some(Commands::Refresh { workspace }) => {
            let config = load_config(cli.config)?;
            let all_running = client.workspaces()?;
            let (running, session) = if let Some(found) = exact_workspace(&all_running, &workspace)?
            {
                let session = config
                    .sessions
                    .values()
                    .find(|session| session.name == workspace)
                    .cloned()
                    .map(Ok)
                    .unwrap_or_else(|| dynamic_session(client, &config, &workspace))?;
                (found, session)
            } else {
                let session = selected(&config, &workspace)?.clone();
                let found = exact_workspace(&all_running, &session.name)?
                    .with_context(|| format!("Workspace '{}' is not running", session.name))?;
                (found, session)
            };
            client.refresh(&running, &session)?;
            println!("✓ Workspace '{}' refreshed", session.name);
            Ok(())
        }
        None => cycle(client, cli.config, inside),
    }
}

fn cycle<R: Runner>(client: &Client<R>, config_path: Option<String>, inside: bool) -> Result<()> {
    let mut running = client.workspaces().unwrap_or_default();
    if running.is_empty() {
        client.ensure_server()?;
        running = client.workspaces().unwrap_or_default();
    }
    if running.is_empty() {
        let config = load_config(config_path)?;
        let session = default_session(&config)?;
        println!("No workspaces running. Creating '{}'...", session.name);
        client.create_workspace(session)?;
    } else {
        let ordered = match load_config(config_path) {
            Ok(config) => configured_order(&config, &running),
            Err(_) => {
                running.sort_by(|a, b| a.label.cmp(&b.label).then(a.id.cmp(&b.id)));
                running
            }
        };
        let current = if inside {
            std::env::var("HERDR_WORKSPACE_ID").ok()
        } else {
            None
        };
        let next = next_workspace(&ordered, current.as_deref());
        client.focus_workspace(&next.id)?;
        println!("Focusing workspace '{}'...", next.label);
    }
    if inside { Ok(()) } else { client.attach() }
}

fn init_config(explicit: Option<String>) -> Result<()> {
    let path = config_path(explicit)?;
    if path.exists() {
        println!("Configuration file already exists at {}", path.display());
        println!("Edit it with: $EDITOR {}", path.display());
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, DEFAULT_CONFIG)?;
    println!(
        "✓ Shared tmx/hmx configuration created at {}",
        path.display()
    );
    println!();
    println!("Edit it with: $EDITOR {}", path.display());
    println!("Then open a workspace with: hmx open dev");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::os::unix::process::ExitStatusExt;
    use std::process::Output;
    use std::sync::{Arc, Mutex};

    type Calls = Arc<Mutex<Vec<(String, Vec<String>)>>>;

    #[derive(Clone)]
    struct FakeRunner {
        outputs: Arc<Mutex<VecDeque<Output>>>,
        calls: Calls,
    }

    impl FakeRunner {
        fn new(outputs: Vec<Output>) -> Self {
            Self {
                outputs: Arc::new(Mutex::new(outputs.into())),
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl Runner for FakeRunner {
        fn output(&self, program: &str, args: &[String]) -> Result<Output> {
            self.calls
                .lock()
                .unwrap()
                .push((program.to_string(), args.to_vec()));
            self.outputs
                .lock()
                .unwrap()
                .pop_front()
                .context("fake output exhausted")
        }

        fn status(&self, program: &str, args: &[String]) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push((program.to_string(), args.to_vec()));
            Ok(())
        }

        fn start_server(&self, program: &str, args: &[String]) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push((format!("{program}:start"), args.to_vec()));
            Ok(())
        }
    }

    fn output(code: i32, stdout: &str, stderr: &str) -> Output {
        Output {
            status: std::process::ExitStatus::from_raw(code << 8),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    fn cli(config: &str, command: Option<Commands>) -> Cli {
        Cli {
            config: Some(config.to_string()),
            remote: None,
            session: None,
            verbose: false,
            command,
        }
    }

    fn test_config(name: &str, contents: &str) -> String {
        let path = std::env::temp_dir().join(format!("hmx-{name}-{}.toml", std::process::id()));
        std::fs::write(&path, contents).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn hmx_config_has_precedence() {
        unsafe {
            std::env::set_var("TMX_CONFIG_PATH", "/tmp/tmx");
            std::env::set_var("HMX_CONFIG_PATH", "/tmp/hmx");
        }
        assert_eq!(config_path(None).unwrap(), PathBuf::from("/tmp/hmx"));
        unsafe {
            std::env::remove_var("TMX_CONFIG_PATH");
            std::env::remove_var("HMX_CONFIG_PATH");
        }
    }

    #[test]
    fn open_exact_running_label_does_not_load_configuration() {
        let runner = FakeRunner::new(vec![
            output(
                0,
                r#"{"result":{"workspaces":[{"workspace_id":"w1","label":"scratch"}]}}"#,
                "",
            ),
            output(0, "", ""),
        ]);
        let calls = runner.calls.clone();
        let client = Client::new(runner, None, None, false);
        run_with_client(
            cli(
                "/definitely/missing.toml",
                Some(Commands::Open {
                    workspace: "scratch".into(),
                }),
            ),
            &client,
            true,
        )
        .unwrap();
        assert_eq!(calls.lock().unwrap()[1].1, ["workspace", "focus", "w1"]);
    }

    #[test]
    fn open_exact_label_takes_precedence_over_configured_key() {
        let path = test_config(
            "precedence",
            "[sessions.scratch]\nname='configured-name'\nwindows=[{name='tab',panes=[{}]}]",
        );
        let runner = FakeRunner::new(vec![
            output(
                0,
                r#"{"workspaces":[{"workspace_id":"live","label":"scratch"}]}"#,
                "",
            ),
            output(0, "", ""),
        ]);
        let calls = runner.calls.clone();
        run_with_client(
            cli(
                &path,
                Some(Commands::Open {
                    workspace: "scratch".into(),
                }),
            ),
            &Client::new(runner, None, None, false),
            true,
        )
        .unwrap();
        assert_eq!(calls.lock().unwrap()[1].1[2], "live");
    }

    #[test]
    fn configured_resolution_rechecks_before_creation() {
        let path = test_config(
            "race",
            "[sessions.api]\nname='api-workspace'\nwindows=[{name='tab',panes=[{}]}]",
        );
        let runner = FakeRunner::new(vec![
            output(0, r#"{"workspaces":[]}"#, ""),
            output(0, r#"{"workspaces":[]}"#, ""),
            output(
                0,
                r#"{"workspaces":[{"workspace_id":"raced","label":"api-workspace"}]}"#,
                "",
            ),
            output(0, "", ""),
        ]);
        let calls = runner.calls.clone();
        run_with_client(
            cli(
                &path,
                Some(Commands::Open {
                    workspace: "api".into(),
                }),
            ),
            &Client::new(runner, None, None, false),
            true,
        )
        .unwrap();
        let calls = calls.lock().unwrap();
        assert_eq!(calls[3].1, ["workspace", "focus", "raced"]);
        assert!(
            !calls
                .iter()
                .any(|(_, args)| args.get(1).is_some_and(|arg| arg == "create"))
        );
    }

    #[test]
    fn dynamic_roots_are_local_cwd_or_remote_home() {
        let config: Config = toml::from_str(
            "default='base'\n[sessions.base]\nname='base'\nroot='/template'\nwindows=[{name='tab',panes=[{}]}]",
        )
        .unwrap();
        let local = dynamic_session(
            &Client::new(FakeRunner::new(vec![]), None, None, false),
            &config,
            "scratch",
        )
        .unwrap();
        assert_eq!(local.name, "scratch");
        assert_eq!(
            local.root,
            std::env::current_dir().unwrap().to_string_lossy()
        );

        let remote = dynamic_session(
            &Client::new(FakeRunner::new(vec![]), Some("workbox".into()), None, false),
            &config,
            "scratch",
        )
        .unwrap();
        assert_eq!(remote.root, "~");
    }

    #[test]
    fn dynamic_creation_uses_local_cwd() {
        let config: Config = toml::from_str(
            "default='base'\n[sessions.base]\nname='base'\nwindows=[{name='tab',panes=[{}]}]",
        )
        .unwrap();
        let runner = FakeRunner::new(vec![
            output(
                0,
                r#"{"workspace_id":"w1","label":"scratch","tab_id":"t1","pane_id":"p1"}"#,
                "",
            ),
            output(0, "", ""),
            output(0, r#"{"panes":[{"pane_id":"p1","tab_id":"t1"}]}"#, ""),
            output(0, "", ""),
            output(0, "", ""),
        ]);
        let calls = runner.calls.clone();
        let client = Client::new(runner, None, None, false);
        let dynamic = dynamic_session(&client, &config, "scratch").unwrap();
        client.create_workspace(&dynamic).unwrap();
        let calls = calls.lock().unwrap();
        let cwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(calls[0].1.windows(2).any(|pair| pair == ["--cwd", &cwd]));
    }

    #[test]
    fn dynamic_remote_creation_expands_root_to_remote_home() {
        let config: Config = toml::from_str(
            "default='base'\n[sessions.base]\nname='base'\nwindows=[{name='tab',panes=[{}]}]",
        )
        .unwrap();
        let runner = FakeRunner::new(vec![
            output(0, "/home/dev", ""),
            output(
                0,
                r#"{"workspace_id":"w1","label":"scratch","tab_id":"t1","pane_id":"p1"}"#,
                "",
            ),
            output(0, "", ""),
            output(0, r#"{"panes":[{"pane_id":"p1","tab_id":"t1"}]}"#, ""),
            output(0, "", ""),
            output(0, "", ""),
        ]);
        let calls = runner.calls.clone();
        let client = Client::new(runner, Some("workbox".into()), None, false);
        let dynamic = dynamic_session(&client, &config, "scratch").unwrap();
        client.create_workspace(&dynamic).unwrap();
        let calls = calls.lock().unwrap();
        assert!(calls[1].1[1].contains("--cwd /home/dev"));
    }

    #[test]
    fn close_exact_unconfigured_label_does_not_load_configuration() {
        let runner = FakeRunner::new(vec![
            output(
                0,
                r#"{"workspaces":[{"workspace_id":"w1","label":"scratch"}]}"#,
                "",
            ),
            output(0, "", ""),
        ]);
        let calls = runner.calls.clone();
        run_with_client(
            cli(
                "/definitely/missing.toml",
                Some(Commands::Close {
                    workspace: "scratch".into(),
                }),
            ),
            &Client::new(runner, None, None, false),
            true,
        )
        .unwrap();
        assert_eq!(calls.lock().unwrap()[1].1, ["workspace", "close", "w1"]);
    }

    #[test]
    fn refresh_unconfigured_running_workspace_uses_default_layout() {
        let path = test_config(
            "refresh-dynamic",
            "default='base'\n[sessions.base]\nname='base'\nwindows=[{name='tab',panes=[{}]}]",
        );
        let runner = FakeRunner::new(vec![
            output(
                0,
                r#"{"workspaces":[{"workspace_id":"w1","label":"scratch"}]}"#,
                "",
            ),
            output(
                0,
                r#"{"tabs":[{"tab_id":"t1","workspace_id":"w1","label":"automatic"}]}"#,
                "",
            ),
            output(0, r#"{"panes":[{"pane_id":"p1","tab_id":"t1"}]}"#, ""),
        ]);
        let calls = runner.calls.clone();
        run_with_client(
            cli(
                &path,
                Some(Commands::Refresh {
                    workspace: "scratch".into(),
                }),
            ),
            &Client::new(runner, None, None, false),
            true,
        )
        .unwrap();
        assert_eq!(calls.lock().unwrap().len(), 3);
    }

    #[test]
    fn cycle_uses_alphabetical_live_order_when_config_is_invalid() {
        let runner = FakeRunner::new(vec![
            output(
                0,
                r#"{"workspaces":[{"workspace_id":"z","label":"zeta"},{"workspace_id":"a","label":"alpha"}]}"#,
                "",
            ),
            output(0, "", ""),
        ]);
        let calls = runner.calls.clone();
        cycle(
            &Client::new(runner, None, None, false),
            Some("/definitely/missing.toml".into()),
            true,
        )
        .unwrap();
        assert_eq!(calls.lock().unwrap()[1].1, ["workspace", "focus", "a"]);
    }

    #[test]
    fn duplicate_live_labels_remain_ambiguous() {
        let running = vec![
            Workspace {
                id: "1".into(),
                label: "same".into(),
            },
            Workspace {
                id: "2".into(),
                label: "same".into(),
            },
        ];
        assert!(
            exact_workspace(&running, "same")
                .unwrap_err()
                .to_string()
                .contains("ambiguous")
        );
    }

    #[test]
    fn dynamic_workspace_requires_a_configured_default() {
        let config: Config =
            toml::from_str("[sessions.base]\nname='base'\nwindows=[{name='tab',panes=[{}]}]")
                .unwrap();
        let error = dynamic_session(
            &Client::new(FakeRunner::new(vec![]), None, None, false),
            &config,
            "scratch",
        )
        .unwrap_err();
        assert!(error.to_string().contains("no default workspace"));
    }

    #[test]
    fn init_existing_config_is_a_successful_noop_and_template_is_shared() {
        let path = test_config("init", "keep me");
        init_config(Some(path.clone())).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "keep me");
        assert!(DEFAULT_CONFIG.contains("[sessions.work]"));
    }

    #[test]
    fn list_uses_configured_ids_and_deduplicates_live_labels() {
        let config: Config = toml::from_str(
            "[sessions.api]\nname='api-workspace'\nwindows=[{name='tab',panes=[{}]}]",
        )
        .unwrap();
        let running = vec![
            Workspace {
                id: "1".into(),
                label: "api-workspace".into(),
            },
            Workspace {
                id: "2".into(),
                label: "scratch".into(),
            },
            Workspace {
                id: "3".into(),
                label: "scratch".into(),
            },
        ];
        assert_eq!(
            list_lines(&config, &running),
            ["Running Herdr workspaces:", "  api (c)", "  scratch"]
        );
    }

    #[test]
    fn validation_reports_layout_and_size_warning() {
        let config: Config = toml::from_str(
            "[sessions.api]\nname='api'\nwindows=[{name='tab',layout='tiled',panes=[{}, {size='30%'}]}]",
        )
        .unwrap();
        assert_eq!(validation_warnings(&config).len(), 1);
        assert!(validation_warnings(&config)[0].contains("sizes will override layout"));
    }
}
