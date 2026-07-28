use crate::config::{Config, Session, Window};
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::collections::HashMap;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Tab {
    id: String,
    workspace_id: String,
    label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PaneRecord {
    id: String,
    tab_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Split {
    pub target_index: usize,
    pub direction: &'static str,
    pub ratio: Option<f64>,
}

pub trait Runner {
    fn output(&self, program: &str, args: &[String]) -> Result<Output>;
    fn status(&self, program: &str, args: &[String]) -> Result<()>;
    fn start_server(&self, program: &str, args: &[String]) -> Result<()>;
}

pub struct SystemRunner;

impl Runner for SystemRunner {
    fn output(&self, program: &str, args: &[String]) -> Result<Output> {
        Command::new(program)
            .args(args)
            .output()
            .with_context(|| format!("Failed to execute {}", program))
    }

    fn status(&self, program: &str, args: &[String]) -> Result<()> {
        let status = Command::new(program)
            .args(args)
            .status()
            .with_context(|| format!("Failed to execute {}", program))?;
        if !status.success() {
            bail!("{} exited with {}", program, status);
        }
        Ok(())
    }

    fn start_server(&self, program: &str, args: &[String]) -> Result<()> {
        Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("Failed to start {} server", program))?;
        Ok(())
    }
}

pub struct Client<R> {
    runner: R,
    remote: Option<String>,
    session: Option<String>,
    verbose: bool,
}

impl<R: Runner> Client<R> {
    pub fn new(runner: R, remote: Option<String>, session: Option<String>, verbose: bool) -> Self {
        Self {
            runner,
            remote,
            session,
            verbose,
        }
    }

    pub fn is_remote(&self) -> bool {
        self.remote.is_some()
    }

    fn herdr_args(&self, command: &[&str]) -> Vec<String> {
        let mut args = Vec::new();
        if let Some(session) = &self.session {
            args.extend(["--session".to_string(), session.clone()]);
        }
        args.extend(command.iter().map(|arg| (*arg).to_string()));
        args
    }

    fn invocation(&self, command: &[&str]) -> (String, Vec<String>) {
        let args = self.herdr_args(command);
        if let Some(target) = &self.remote {
            (
                "ssh".to_string(),
                vec![target.clone(), shell_join("herdr", &args)],
            )
        } else {
            ("herdr".to_string(), args)
        }
    }

    fn command_output(&self, command: &[&str]) -> Result<Output> {
        let (program, args) = self.invocation(command);
        self.trace(&program, &args);
        let output = self.runner.output(&program, &args)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if self.remote.is_some()
                && (output.status.code() == Some(127) || stderr.contains("not found"))
            {
                bail!(
                    "Herdr is not installed on the remote host. Install it from https://herdr.dev/docs/install/"
                );
            }
            bail!("Herdr command failed: {}", stderr);
        }
        Ok(output)
    }

    fn output(&self, command: &[&str]) -> Result<Value> {
        let output = self.command_output(command)?;
        serde_json::from_slice(&output.stdout).with_context(|| {
            format!(
                "Herdr returned invalid JSON: {}",
                String::from_utf8_lossy(&output.stdout)
            )
        })
    }

    fn mutation(&self, command: &[&str]) -> Result<Value> {
        let output = self.command_output(command)?;
        Ok(serde_json::from_slice(&output.stdout).unwrap_or(Value::Null))
    }

    fn trace(&self, program: &str, args: &[String]) {
        if self.verbose {
            eprintln!(
                "{} {}",
                program,
                args.iter()
                    .map(|a| shell_quote(a))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
    }

    pub fn workspaces(&self) -> Result<Vec<Workspace>> {
        Ok(parse_workspaces(&self.output(&["workspace", "list"])?))
    }

    pub fn ensure_server(&self) -> Result<()> {
        if self.workspaces().is_ok() {
            return Ok(());
        }
        if let Some(target) = &self.remote {
            let check_args = vec![
                target.clone(),
                "command -v herdr >/dev/null 2>&1".to_string(),
            ];
            self.trace("ssh", &check_args);
            if !self.runner.output("ssh", &check_args)?.status.success() {
                bail!(
                    "Herdr is not installed on the remote host. Install it from https://herdr.dev/docs/install/"
                );
            }
            let args = self.herdr_args(&["server"]);
            let remote_command = format!("nohup {} >/dev/null 2>&1 &", shell_join("herdr", &args));
            let ssh_args = vec![target.clone(), remote_command];
            self.trace("ssh", &ssh_args);
            self.runner.status("ssh", &ssh_args)?;
        } else {
            let args = self.herdr_args(&["server"]);
            self.trace("herdr", &args);
            self.runner.start_server("herdr", &args)?;
        }
        for _ in 0..20 {
            if self.workspaces().is_ok() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(50));
        }
        bail!("Herdr server did not become ready");
    }

    pub fn remote_home(&self) -> Result<Option<String>> {
        let Some(target) = &self.remote else {
            return Ok(None);
        };
        let args = vec![target.clone(), "printf '%s' \"$HOME\"".to_string()];
        self.trace("ssh", &args);
        let output = self.runner.output("ssh", &args)?;
        if !output.status.success() {
            bail!(
                "Failed to query remote home directory: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ))
    }

    fn expand_path(&self, path: &str, remote_home: Option<&str>) -> String {
        if let Some(home) = remote_home {
            if path == "~" {
                return home.to_string();
            }
            if let Some(rest) = path.strip_prefix("~/") {
                return format!("{home}/{rest}");
            }
            path.to_string()
        } else {
            shellexpand::tilde(path).to_string()
        }
    }

    pub fn create_workspace(&self, session: &Session) -> Result<Workspace> {
        session.validate()?;
        let remote_home = self.remote_home()?;
        let workspace_root = self.expand_path(&session.root, remote_home.as_deref());
        let first_window = &session.windows[0];
        let first_root = self.expand_path(
            &first_window
                .root
                .clone()
                .unwrap_or_else(|| workspace_root.clone()),
            remote_home.as_deref(),
        );
        let mut command = vec![
            "workspace".to_string(),
            "create".to_string(),
            "--cwd".to_string(),
            first_root,
            "--label".to_string(),
            session.name.clone(),
            "--no-focus".to_string(),
        ];
        add_env(&mut command, &first_window.panes[0].env);
        let refs: Vec<_> = command.iter().map(String::as_str).collect();
        let created = self.mutation(&refs)?;
        let workspace = parse_created_workspace(&created)?;
        let first_tab_id =
            find_string(&created, "tab_id").context("Herdr create response omitted tab_id")?;
        let first_pane_id =
            find_string(&created, "pane_id").context("Herdr create response omitted pane_id")?;
        self.rename_tab(&first_tab_id, &first_window.name)?;
        let mut startup_tab = first_tab_id.clone();
        let mut startup_window = first_window;
        self.populate_tab(
            &workspace.id,
            &first_tab_id,
            &first_pane_id,
            first_window,
            &workspace_root,
            remote_home.as_deref(),
        )?;
        if session.resolve_startup_window() == 0 {
            let _ = self.pane_at(&first_tab_id, session.get_startup_pane())?;
        }
        for (index, window) in session.windows.iter().enumerate().skip(1) {
            let root = self.expand_path(
                &window
                    .root
                    .clone()
                    .unwrap_or_else(|| workspace_root.clone()),
                remote_home.as_deref(),
            );
            let mut args = vec![
                "tab".to_string(),
                "create".to_string(),
                "--workspace".to_string(),
                workspace.id.clone(),
                "--cwd".to_string(),
                root,
                "--label".to_string(),
                window.name.clone(),
                "--no-focus".to_string(),
            ];
            add_env(&mut args, &window.panes[0].env);
            let refs: Vec<_> = args.iter().map(String::as_str).collect();
            let response = self.mutation(&refs)?;
            let tab_id = find_string(&response, "tab_id")
                .context("Herdr tab create response omitted tab_id")?;
            let pane_id = find_string(&response, "pane_id")
                .context("Herdr tab create response omitted pane_id")?;
            self.populate_tab(
                &workspace.id,
                &tab_id,
                &pane_id,
                window,
                &workspace_root,
                remote_home.as_deref(),
            )?;
            if session.resolve_startup_window() == index {
                startup_tab = tab_id.clone();
                startup_window = window;
            }
        }
        self.run_command(&first_pane_id, &first_window.panes[0].command)?;
        self.focus(
            &workspace.id,
            &startup_tab,
            startup_window,
            session.get_startup_pane(),
        )?;
        Ok(workspace)
    }

    fn populate_tab(
        &self,
        _workspace_id: &str,
        tab_id: &str,
        root_pane: &str,
        window: &Window,
        workspace_root: &str,
        remote_home: Option<&str>,
    ) -> Result<()> {
        let plan = layout_plan(window);
        let window_root = window.root.as_deref().unwrap_or(workspace_root);
        let mut pane_ids = vec![root_pane.to_string()];
        for index in 1..window.panes.len() {
            let pane = &window.panes[index];
            let split = &plan[index - 1];
            let pane_root =
                self.expand_path(pane.root.as_deref().unwrap_or(window_root), remote_home);
            let mut args = vec![
                "pane".to_string(),
                "split".to_string(),
                pane_ids[split.target_index].clone(),
                "--direction".to_string(),
                split.direction.to_string(),
                "--cwd".to_string(),
                pane_root,
                "--no-focus".to_string(),
            ];
            if let Some(ratio) = split.ratio {
                args.extend(["--ratio".to_string(), format!("{ratio:.6}")]);
            }
            add_env(&mut args, &pane.env);
            let refs: Vec<_> = args.iter().map(String::as_str).collect();
            let response = self.mutation(&refs)?;
            let pane_id = find_string(&response, "pane_id")
                .context("Herdr split response omitted pane_id")?;
            self.run_command(&pane_id, &pane.command)?;
            pane_ids.push(pane_id);
        }
        let _ = tab_id;
        Ok(())
    }

    fn run_command(&self, pane_id: &str, command: &str) -> Result<()> {
        if command.is_empty() {
            return Ok(());
        }
        self.mutation(&["pane", "run", pane_id, command])?;
        Ok(())
    }

    fn rename_tab(&self, tab_id: &str, label: &str) -> Result<()> {
        self.mutation(&["tab", "rename", tab_id, label])?;
        Ok(())
    }

    fn pane_at(&self, tab_id: &str, index: usize) -> Result<String> {
        let panes = parse_panes(&self.output(&["pane", "list"])?);
        panes
            .into_iter()
            .filter(|pane| pane.tab_id == tab_id)
            .nth(index)
            .map(|pane| pane.id)
            .with_context(|| format!("Startup pane {} does not exist in tab", index))
    }

    fn focus(
        &self,
        workspace_id: &str,
        tab_id: &str,
        window: &Window,
        pane_index: usize,
    ) -> Result<()> {
        self.mutation(&["workspace", "focus", workspace_id])?;
        self.mutation(&["tab", "focus", tab_id])?;
        if pane_index > 0 {
            let panes = parse_panes(&self.output(&["pane", "list"])?);
            let ids: Vec<_> = panes
                .into_iter()
                .filter(|pane| pane.tab_id == tab_id)
                .map(|pane| pane.id)
                .collect();
            let split = &layout_plan(window)[pane_index - 1];
            let parent = ids
                .get(split.target_index)
                .context("Startup pane parent does not exist")?;
            self.mutation(&[
                "pane",
                "focus",
                "--pane",
                parent,
                "--direction",
                split.direction,
            ])?;
        }
        Ok(())
    }

    pub fn focus_workspace(&self, id: &str) -> Result<()> {
        self.mutation(&["workspace", "focus", id])?;
        Ok(())
    }

    pub fn close_workspace(&self, id: &str) -> Result<()> {
        self.mutation(&["workspace", "close", id])?;
        Ok(())
    }

    pub fn refresh(&self, workspace: &Workspace, session: &Session) -> Result<()> {
        session.validate()?;
        let remote_home = self.remote_home()?;
        let tabs = parse_tabs(&self.output(&["tab", "list", "--workspace", &workspace.id])?);
        let panes = parse_panes(&self.output(&["pane", "list", "--workspace", &workspace.id])?);
        let workspace_root = self.expand_path(&session.root, remote_home.as_deref());
        for (window_index, window) in session.windows.iter().enumerate() {
            // tmux refresh addresses windows by index. Herdr tabs may be automatically renamed,
            // so position is the stable equivalent and avoids creating duplicate tabs.
            if let Some(tab) = tabs.get(window_index) {
                let existing: Vec<_> = panes.iter().filter(|pane| pane.tab_id == tab.id).collect();
                if existing.len() < window.panes.len() {
                    let plan = layout_plan(window);
                    let mut ids: Vec<_> = existing.iter().map(|pane| pane.id.clone()).collect();
                    for index in existing.len()..window.panes.len() {
                        let pane = &window.panes[index];
                        let split = &plan[index - 1];
                        let root = self.expand_path(
                            pane.root
                                .as_deref()
                                .or(window.root.as_deref())
                                .unwrap_or(&workspace_root),
                            remote_home.as_deref(),
                        );
                        let mut args = vec![
                            "pane".to_string(),
                            "split".to_string(),
                            ids[split.target_index.min(ids.len() - 1)].clone(),
                            "--direction".to_string(),
                            split.direction.to_string(),
                            "--cwd".to_string(),
                            root,
                            "--no-focus".to_string(),
                        ];
                        if let Some(ratio) = split.ratio {
                            args.extend(["--ratio".to_string(), format!("{ratio:.6}")]);
                        }
                        add_env(&mut args, &pane.env);
                        let refs: Vec<_> = args.iter().map(String::as_str).collect();
                        let response = self.mutation(&refs)?;
                        let id = find_string(&response, "pane_id")
                            .context("Herdr split response omitted pane_id")?;
                        self.run_command(&id, &pane.command)?;
                        ids.push(id);
                    }
                }
            } else {
                let root = self.expand_path(
                    window.root.as_deref().unwrap_or(&workspace_root),
                    remote_home.as_deref(),
                );
                let mut args = vec![
                    "tab".to_string(),
                    "create".to_string(),
                    "--workspace".to_string(),
                    workspace.id.clone(),
                    "--cwd".to_string(),
                    root,
                    "--label".to_string(),
                    window.name.clone(),
                    "--no-focus".to_string(),
                ];
                add_env(&mut args, &window.panes[0].env);
                let refs: Vec<_> = args.iter().map(String::as_str).collect();
                let response = self.mutation(&refs)?;
                let tab_id = find_string(&response, "tab_id")
                    .context("Herdr tab create response omitted tab_id")?;
                let pane_id = find_string(&response, "pane_id")
                    .context("Herdr tab create response omitted pane_id")?;
                self.populate_tab(
                    &workspace.id,
                    &tab_id,
                    &pane_id,
                    window,
                    &workspace_root,
                    remote_home.as_deref(),
                )?;
                self.run_command(&pane_id, &window.panes[0].command)?;
            }
        }
        Ok(())
    }

    pub fn attach(&self) -> Result<()> {
        let mut args = Vec::new();
        if let Some(target) = &self.remote {
            args.extend(["--remote".to_string(), target.clone()]);
        }
        if let Some(session) = &self.session {
            args.extend(["--session".to_string(), session.clone()]);
        }
        self.trace("herdr", &args);
        self.runner.status("herdr", &args)
    }
}

pub fn configured_order(config: &Config, running: &[Workspace]) -> Vec<Workspace> {
    let mut result = Vec::new();
    let mut included = std::collections::HashSet::new();
    for id in config.session_ids() {
        if let Some(session) = config.sessions.get(&id) {
            for workspace in running.iter().filter(|ws| ws.label == session.name) {
                if included.insert(workspace.id.clone()) {
                    result.push(workspace.clone());
                }
            }
        }
    }
    let configured: Vec<_> = config
        .sessions
        .values()
        .map(|session| &session.name)
        .collect();
    let mut extras: Vec<_> = running
        .iter()
        .filter(|ws| !configured.contains(&&ws.label) && included.insert(ws.id.clone()))
        .cloned()
        .collect();
    extras.sort_by(|a, b| a.label.cmp(&b.label).then(a.id.cmp(&b.id)));
    result.extend(extras);
    result
}

pub fn next_workspace<'a>(ordered: &'a [Workspace], current_id: Option<&str>) -> &'a Workspace {
    let index = current_id
        .and_then(|id| ordered.iter().position(|ws| ws.id == id))
        .map(|index| (index + 1) % ordered.len())
        .unwrap_or(0);
    &ordered[index]
}

pub fn layout_plan(window: &Window) -> Vec<Split> {
    let count = window.panes.len();
    (1..count).map(|index| {
        let pane = &window.panes[index];
        let layout = window.layout.as_deref().unwrap_or(if count == 2 { "even-horizontal" } else { "tiled" });
        let (target_index, direction, default_ratio) = match layout {
            "even-horizontal" => (index - 1, "right", Some(1.0 / (count - index + 1) as f64)),
            "even-vertical" => (index - 1, "down", Some(1.0 / (count - index + 1) as f64)),
            "main-vertical" => (if index == 1 { 0 } else { index - 1 }, if index == 1 { "right" } else { "down" }, Some(if index == 1 { 0.33 } else { 1.0 / (count - index + 1) as f64 })),
            "main-horizontal" => (if index == 1 { 0 } else { index - 1 }, if index == 1 { "down" } else { "right" }, Some(if index == 1 { 0.33 } else { 1.0 / (count - index + 1) as f64 })),
            _ => ((index - 1) / 2, if index % 2 == 1 { "right" } else { "down" }, Some(0.5)),
        };
        let explicit_direction = pane.split.as_deref().map(|split| if split == "horizontal" { "right" } else { "down" });
        let ratio = pane.size.as_deref().and_then(percent_ratio).or(default_ratio);
        if pane.size.as_deref().is_some_and(|size| !size.ends_with('%')) {
            eprintln!("Warning: ignoring absolute pane size '{}' for Herdr; only ratios are representable", pane.size.as_deref().unwrap());
        }
        Split { target_index, direction: explicit_direction.unwrap_or(direction), ratio }
    }).collect()
}

fn percent_ratio(size: &str) -> Option<f64> {
    size.strip_suffix('%')?
        .parse::<f64>()
        .ok()
        .map(|value| value / 100.0)
}

fn add_env(args: &mut Vec<String>, env: &HashMap<String, String>) {
    let mut pairs: Vec<_> = env.iter().collect();
    pairs.sort_by_key(|(key, _)| *key);
    for (key, value) in pairs {
        args.extend(["--env".to_string(), format!("{key}={value}")]);
    }
}

fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_./:@%+=".contains(&b))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
}

fn shell_join(program: &str, args: &[String]) -> String {
    std::iter::once(shell_quote(program))
        .chain(args.iter().map(|arg| shell_quote(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn find_string(value: &Value, key: &str) -> Option<String> {
    match value {
        Value::Object(map) => map
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| map.values().find_map(|value| find_string(value, key))),
        Value::Array(values) => values.iter().find_map(|value| find_string(value, key)),
        _ => None,
    }
}

fn records(value: &Value, collection: &str, id_key: &str) -> Vec<Value> {
    if let Some(values) = value
        .get("result")
        .and_then(|v| v.get(collection))
        .and_then(Value::as_array)
    {
        return values.clone();
    }
    if let Some(values) = value.get(collection).and_then(Value::as_array) {
        return values.clone();
    }
    let mut found = Vec::new();
    match value {
        Value::Object(map) => {
            if map.contains_key(id_key) {
                found.push(value.clone());
            } else {
                for child in map.values() {
                    found.extend(records(child, collection, id_key));
                }
            }
        }
        Value::Array(values) => {
            for child in values {
                found.extend(records(child, collection, id_key));
            }
        }
        _ => {}
    }
    found
}

pub fn parse_workspaces(value: &Value) -> Vec<Workspace> {
    records(value, "workspaces", "workspace_id")
        .into_iter()
        .filter_map(|value| {
            Some(Workspace {
                id: value.get("workspace_id")?.as_str()?.to_string(),
                label: value
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .collect()
}

fn parse_created_workspace(value: &Value) -> Result<Workspace> {
    let workspace = value
        .pointer("/result/workspace")
        .or_else(|| value.get("workspace"))
        .unwrap_or(value);
    let id = workspace
        .get("workspace_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| find_string(value, "workspace_id"))
        .context("Herdr create response omitted workspace_id")?;
    let label = workspace
        .get("label")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_default();
    Ok(Workspace { id, label })
}

fn parse_tabs(value: &Value) -> Vec<Tab> {
    records(value, "tabs", "tab_id")
        .into_iter()
        .filter_map(|value| {
            Some(Tab {
                id: value.get("tab_id")?.as_str()?.to_string(),
                workspace_id: value
                    .get("workspace_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                label: value
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .collect()
}

fn parse_panes(value: &Value) -> Vec<PaneRecord> {
    records(value, "panes", "pane_id")
        .into_iter()
        .filter_map(|value| {
            Some(PaneRecord {
                id: value.get("pane_id")?.as_str()?.to_string(),
                tab_id: value
                    .get("tab_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Pane, Window};
    use std::collections::VecDeque;
    use std::os::unix::process::ExitStatusExt;
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

    fn window(layout: Option<&str>, count: usize) -> Window {
        Window {
            name: "tab".into(),
            panes: (0..count)
                .map(|_| Pane {
                    command: String::new(),
                    env: HashMap::new(),
                    root: None,
                    split: None,
                    size: None,
                })
                .collect(),
            layout: layout.map(str::to_string),
            root: None,
        }
    }

    #[test]
    fn parses_json_shapes() {
        let value =
            serde_json::json!({"result":{"workspaces":[{"workspace_id":"w1","label":"api"}]}});
        assert_eq!(
            parse_workspaces(&value),
            vec![Workspace {
                id: "w1".into(),
                label: "api".into()
            }]
        );
    }

    #[test]
    fn parses_workspace_label_in_create_response_instead_of_tab_label() {
        let value = serde_json::json!({
            "result": {
                "tab": {"label": "1", "tab_id": "w1:t1", "workspace_id": "w1"},
                "workspace": {"label": "scratch", "workspace_id": "w1"}
            }
        });
        assert_eq!(
            parse_created_workspace(&value).unwrap(),
            Workspace {
                id: "w1".into(),
                label: "scratch".into()
            }
        );
    }

    #[test]
    fn ordering_and_cycle_wrap() {
        let config: Config = toml::from_str("[sessions.b]\nname='beta'\nwindows=[{name='x',panes=[{}]}]\n[sessions.a]\nname='alpha'\nwindows=[{name='x',panes=[{}]}]").unwrap();
        let running = vec![
            Workspace {
                id: "3".into(),
                label: "extra".into(),
            },
            Workspace {
                id: "2".into(),
                label: "beta".into(),
            },
            Workspace {
                id: "1".into(),
                label: "alpha".into(),
            },
        ];
        let ordered = configured_order(&config, &running);
        assert_eq!(
            ordered.iter().map(|w| w.label.as_str()).collect::<Vec<_>>(),
            ["alpha", "beta", "extra"]
        );
        assert_eq!(next_workspace(&ordered, Some("3")).id, "1");
    }

    #[test]
    fn quotes_remote_shell_arguments() {
        assert_eq!(shell_quote("a'b c"), "'a'\"'\"'b c'");
        assert_eq!(
            shell_join(
                "herdr",
                &[
                    "pane".into(),
                    "run".into(),
                    "p1".into(),
                    "echo $HOME".into()
                ]
            ),
            "herdr pane run p1 'echo $HOME'"
        );
    }

    #[test]
    fn translates_all_layouts() {
        assert!(
            layout_plan(&window(Some("even-horizontal"), 4))
                .iter()
                .all(|s| s.direction == "right")
        );
        assert!(
            layout_plan(&window(Some("even-vertical"), 4))
                .iter()
                .all(|s| s.direction == "down")
        );
        assert_eq!(
            layout_plan(&window(Some("main-vertical"), 4))
                .iter()
                .map(|s| s.direction)
                .collect::<Vec<_>>(),
            ["right", "down", "down"]
        );
        assert_eq!(
            layout_plan(&window(Some("main-horizontal"), 4))
                .iter()
                .map(|s| s.direction)
                .collect::<Vec<_>>(),
            ["down", "right", "right"]
        );
        assert_eq!(
            layout_plan(&window(Some("tiled"), 4))
                .iter()
                .map(|s| s.direction)
                .collect::<Vec<_>>(),
            ["right", "down", "right"]
        );
        assert_eq!(layout_plan(&window(None, 3)).len(), 2);
    }

    #[test]
    fn explicit_split_and_percentage_override_layout() {
        let mut value = window(Some("even-vertical"), 2);
        value.panes[1].split = Some("horizontal".into());
        value.panes[1].size = Some("30%".into());
        assert_eq!(
            layout_plan(&value),
            vec![Split {
                target_index: 0,
                direction: "right",
                ratio: Some(0.3)
            }]
        );
    }

    #[test]
    fn remote_calls_are_quoted_and_use_local_ssh_client() {
        let runner = FakeRunner::new(vec![
            output(0, "/home/dev", ""),
            output(
                0,
                r#"{"result":{"workspaces":[{"workspace_id":"w1","label":"api"}]}}"#,
                "",
            ),
        ]);
        let calls = runner.calls.clone();
        let client = Client::new(
            runner,
            Some("dev@workbox".into()),
            Some("agents".into()),
            false,
        );
        assert_eq!(client.remote_home().unwrap(), Some("/home/dev".into()));
        assert_eq!(client.workspaces().unwrap()[0].label, "api");
        let calls = calls.lock().unwrap();
        assert_eq!(
            calls[0],
            (
                "ssh".into(),
                vec!["dev@workbox".into(), "printf '%s' \"$HOME\"".into()]
            )
        );
        assert_eq!(
            calls[1],
            (
                "ssh".into(),
                vec![
                    "dev@workbox".into(),
                    "herdr --session agents workspace list".into()
                ]
            )
        );
    }

    #[test]
    fn local_server_bootstrap_retries_then_succeeds() {
        let runner = FakeRunner::new(vec![
            output(1, "", "socket missing"),
            output(0, r#"{"result":{"workspaces":[]}}"#, ""),
        ]);
        let calls = runner.calls.clone();
        Client::new(runner, None, Some("work".into()), false)
            .ensure_server()
            .unwrap();
        let calls = calls.lock().unwrap();
        assert_eq!(
            calls[1],
            (
                "herdr:start".into(),
                vec!["--session".into(), "work".into(), "server".into()]
            )
        );
    }

    #[test]
    fn successful_mutation_can_return_non_json() {
        let runner = FakeRunner::new(vec![output(0, "error", "")]);
        Client::new(runner, None, None, false)
            .focus_workspace("w1")
            .unwrap();
    }

    #[test]
    fn refresh_matches_automatically_renamed_tabs_by_position() {
        let runner = FakeRunner::new(vec![
            output(
                0,
                r#"{"result":{"tabs":[{"tab_id":"t1","workspace_id":"w1","label":"nvim"},{"tab_id":"t2","workspace_id":"w1","label":"shell"}]}}"#,
                "",
            ),
            output(
                0,
                r#"{"result":{"panes":[{"pane_id":"p1","tab_id":"t1"},{"pane_id":"p2","tab_id":"t2"},{"pane_id":"extra","tab_id":"t2"}]}}"#,
                "",
            ),
        ]);
        let calls = runner.calls.clone();
        let session: Session = toml::from_str(
            r#"name = "dev"
root = "/tmp"
windows = [
  { name = "editor", panes = [{}] },
  { name = "terminal", panes = [{}] },
]"#,
        )
        .unwrap();
        Client::new(runner, None, None, false)
            .refresh(
                &Workspace {
                    id: "w1".into(),
                    label: "dev".into(),
                },
                &session,
            )
            .unwrap();
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 2, "renamed tabs and extra panes are preserved");
    }

    #[test]
    fn refresh_adds_only_missing_positional_panes() {
        let runner = FakeRunner::new(vec![
            output(
                0,
                r#"{"result":{"tabs":[{"tab_id":"t1","workspace_id":"w1","label":"automatic"}]}}"#,
                "",
            ),
            output(
                0,
                r#"{"result":{"panes":[{"pane_id":"p1","tab_id":"t1"}]}}"#,
                "",
            ),
            output(0, r#"{"pane_id":"p2"}"#, ""),
        ]);
        let calls = runner.calls.clone();
        let session: Session = toml::from_str(
            r#"name = "dev"
root = "/tmp"
windows = [{ name = "editor", panes = [{}, {}] }]"#,
        )
        .unwrap();
        Client::new(runner, None, None, false)
            .refresh(
                &Workspace {
                    id: "w1".into(),
                    label: "dev".into(),
                },
                &session,
            )
            .unwrap();
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[2].1[0..3], ["pane", "split", "p1"]);
    }
}
