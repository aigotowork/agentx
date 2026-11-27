//! Originating dotenv selection, composition and value-free inspection.
//!
//! Selectors are frozen from CLI, caller environment and the pre-dotenv control
//! scope. Resolve runs before configuration/Provider/IPC acquisition. Only the
//! single-threaded originating entry installs its result; a shared daemon never
//! installs a request's environment. See `.docs/architecture/environment.md`.

use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const FILES: &str = "AGENTX_ENV_FILES";
const OVERRIDE: &str = "AGENTX_ENV_OVERRIDE";
const RETAIN: &str = "AGENTX_ENV_RETAIN_KEYS";
const SELECTORS: [&str; 3] = [FILES, OVERRIDE, RETAIN];
// Existing options with opaque values must not be mistaken for our flags.
const VALUE_FLAGS: &[&str] = &[
    "--profile",
    "--tool",
    "--count",
    "--iam",
    "--name",
    "--terminal",
    "--device",
    "--in",
    "--message",
    "--dir",
    "--workspace",
    "--project",
    "--scope",
    "--agentx-system-prompt",
    "--system",
    "--agentx-prompt",
    "--batch-id",
    "--label",
    "--if-revision",
    "--request-id",
    "--bind",
    "--assets",
    "--host",
    "--port",
    "--payload",
    "--file",
    "--from",
    "--title",
    "--description",
    "--hints",
    "--after",
    "--before",
    "--format",
    "--filter",
    "--work",
    "--generation",
    "--command-id",
    "--assignment",
    "--brief",
    "--revision",
    "--limit",
    "--window-file",
    "-i",
];

/// Outer scanners must skip option values, including text resembling controls.
pub(crate) fn takes_opaque_value(option: &str) -> bool {
    VALUE_FLAGS.contains(&option) || option == "--env-file"
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CliSelection {
    pub files: Option<Vec<String>>,
    pub override_values: Option<bool>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SelectorSource {
    Cli,
    Environment,
    ScopedConfig,
    Default,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct SourceReport {
    pub path: PathBuf,
    pub status: &'static str,
    pub variables: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "source", rename_all = "snake_case")]
pub(crate) enum VariableProvenance {
    Inherited,
    File { index: usize },
}

/// Metadata only: no variable values, raw dotenv text or parser errors.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct EnvironmentReceipt {
    pub project: PathBuf,
    pub selector_scope: PathBuf,
    pub scope: PathBuf,
    pub files_source: SelectorSource,
    pub override_source: SelectorSource,
    pub override_values: bool,
    pub sources: Vec<SourceReport>,
    pub variables: BTreeMap<String, VariableProvenance>,
    pub withheld_keys: BTreeSet<String>,
}

pub(crate) struct ResolvedEnvironment {
    pub values: HashMap<String, String>,
    pub receipt: EnvironmentReceipt,
    pub originating_keys: BTreeSet<String>,
}

#[derive(Default)]
struct ProcessState {
    receipt: Option<EnvironmentReceipt>,
    originating_keys: BTreeSet<String>,
}
fn process_state() -> &'static Mutex<ProcessState> {
    static STATE: OnceLock<Mutex<ProcessState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(ProcessState::default()))
}

/// Called before any threads or scope acquisition. Internal service/runner
/// processes and metadata-only requests deliberately bypass this entry.
pub(crate) fn prepare(argv: &[String]) -> Result<()> {
    let base = std::env::vars().collect::<HashMap<_, _>>();
    let cwd = std::env::current_dir().context("cannot resolve originating working directory")?;
    let resolved = resolve(&base, argv, &cwd)?;
    for (key, value) in &resolved.values {
        // SAFETY: main calls this only during single-threaded host bootstrap.
        unsafe { std::env::set_var(key, value) };
    }
    *process_state().lock().unwrap_or_else(|error| error.into_inner()) = ProcessState { receipt: Some(resolved.receipt), originating_keys: resolved.originating_keys };
    Ok(())
}

/// Extract only environment controls, preserving ordinary global flags, help
/// and all opaque values/arguments after `--`. Last override flag wins.
pub(crate) fn extract_options(argv: &[String]) -> Result<(Vec<String>, CliSelection)> {
    let mut out = Vec::with_capacity(argv.len());
    let mut selection = CliSelection::default();
    let mut i = 0;
    while i < argv.len() {
        let arg = argv[i].as_str();
        if arg == "--" {
            out.extend_from_slice(&argv[i..]);
            break;
        }
        let file = if arg == "--env-file" {
            let value = argv.get(i + 1).context("--env-file requires a path")?;
            if value.starts_with("--") {
                bail!("--env-file requires a path (use --env-file=PATH for a path beginning with --)");
            }
            i += 2;
            Some(value.as_str())
        } else if let Some(value) = arg.strip_prefix("--env-file=") {
            i += 1;
            Some(value)
        } else {
            None
        };
        if let Some(file) = file {
            selection.files.get_or_insert_with(Vec::new).push(file.to_string());
            continue;
        }
        if matches!(arg, "--env-override" | "--no-env-override") {
            selection.override_values = Some(arg == "--env-override");
            i += 1;
            continue;
        }
        out.push(argv[i].clone());
        i += 1;
        if VALUE_FLAGS.contains(&arg) && i < argv.len() {
            out.push(argv[i].clone());
            i += 1;
        }
    }
    if let Some(files) = &mut selection.files {
        if files.len() == 1 && files[0].is_empty() {
            files.clear();
        } else if files.iter().any(String::is_empty) {
            bail!("an empty --env-file selector cannot be combined with other files");
        }
    }
    Ok((out, selection))
}

/// Metadata and internal process entries do not load dotenv files. Native
/// stdio project probes retain their earlier explicit-dir direnv behavior.
pub(crate) fn is_lightweight(argv: &[String]) -> bool {
    if crate::cli::commands::help::all_requested(argv) {
        return true;
    }
    let Ok((args, _)) = extract_options(argv) else {
        return false;
    };
    if matches!(args.first().map(String::as_str), Some("help" | "--help" | "-h" | "--version" | "-v" | "pty" | "relay-worker" | "__update-check")) {
        return true;
    }
    if args.first().map(String::as_str) == Some("daemon") && args.get(1).map(String::as_str) == Some("serve") {
        return true;
    }
    let (words, _, _) = crate::cli::router::extract_global_flags_full(&args);
    if matches!(words.first().map(String::as_str), Some("docs" | "profile")) {
        return true;
    }
    let mut i = 0;
    while i < args.len() && args[i] != "--" {
        if matches!(args[i].as_str(), "--help" | "-h") {
            return true;
        }
        i += if VALUE_FLAGS.contains(&args[i].as_str()) { 2 } else { 1 };
    }
    crate::cli::commands::launch::stdio_project_request(&args).ok().flatten().is_some_and(|request| request.version)
}

/// Resolve explicit source data without changing process state. Scope controls
/// come from the pre-dotenv scope; a file can never recursively change them.
pub(crate) fn resolve(base: &HashMap<String, String>, argv: &[String], cwd: &Path) -> Result<ResolvedEnvironment> {
    let (args, cli) = extract_options(argv)?;
    let (project, explicit_scope) = selected_paths(&args, base, cwd)?;
    let selector_scope = explicit_scope.clone().unwrap_or_else(|| crate::paths::resolve_agentx_dir_from_env(base, cwd).0);
    let want_files = cli.files.is_none() && !base.contains_key(FILES);
    let want_override = cli.override_values.is_none() && !base.contains_key(OVERRIDE);
    let config = crate::config::read_environment_settings(&selector_scope.join("config.toml"), want_files, want_override).map_err(anyhow::Error::msg)?;
    let (files, files_source) = if let Some(files) = cli.files {
        (files, SelectorSource::Cli)
    } else if let Some(value) = base.get(FILES) {
        (split_file_list(value), SelectorSource::Environment)
    } else if let Some(files) = config.files {
        (files, SelectorSource::ScopedConfig)
    } else {
        (Vec::new(), SelectorSource::Default)
    };
    let (override_values, override_source) = if let Some(value) = cli.override_values {
        (value, SelectorSource::Cli)
    } else if let Some(value) = base.get(OVERRIDE) {
        (parse_bool(value)?, SelectorSource::Environment)
    } else if let Some(value) = config.override_values {
        (value, SelectorSource::ScopedConfig)
    } else {
        (false, SelectorSource::Default)
    };
    let mut result = ResolvedEnvironment {
        values: base.clone(),
        originating_keys: retained_keys(base.get(RETAIN).map(String::as_str)),
        receipt: EnvironmentReceipt { project: project.clone(), selector_scope, scope: PathBuf::new(), files_source, override_source, override_values, sources: Vec::new(), variables: BTreeMap::new(), withheld_keys: BTreeSet::new() },
    };
    for raw in files {
        if raw.is_empty() {
            bail!("environment.files contains an empty path");
        }
        if Path::new(&raw).file_name().is_some_and(|name| name == ".envrc") {
            bail!(".envrc requires authorized direnv and cannot be loaded as dotenv: {raw}");
        }
        let path = expand_path(&project, &raw, base)?;
        let entries = parse_dotenv(&path)?;
        let index = result.receipt.sources.len() + 1;
        let variables = entries.len();
        for (key, value) in entries {
            if SELECTORS.contains(&key.as_str()) {
                result.receipt.withheld_keys.insert(key);
                continue;
            }
            if key == "AGENTX_DIR" && files_source == SelectorSource::ScopedConfig {
                bail!("scoped environment file cannot declare AGENTX_DIR: {}", path.display());
            }
            if key == "AGENTX_DIR" && explicit_scope.is_some() {
                result.receipt.withheld_keys.insert(key);
                continue;
            }
            result.originating_keys.insert(key.clone());
            if !override_values && contains_key(base, &key) {
                result.receipt.variables.insert(key, VariableProvenance::Inherited);
            } else {
                insert_value(&mut result.values, key.clone(), value);
                result.receipt.variables.insert(key, VariableProvenance::File { index });
            }
        }
        result.receipt.sources.push(SourceReport { path, status: "loaded", variables });
    }
    let scope = explicit_scope.unwrap_or_else(|| crate::paths::resolve_agentx_dir_from_env(&result.values, cwd).0);
    // Keep the caller snapshot literal. Explicit diagnostic/workbench scope
    // selection does not rewrite its AGENTX_DIR claim; the adapter owns that
    // explicit scope. Ordinary clients resolve relative paths at this cwd.
    result.receipt.scope = scope.canonicalize().unwrap_or(scope);
    if !result.originating_keys.is_empty() {
        result.values.insert(RETAIN.into(), serde_json::to_string(&result.originating_keys)?);
    }
    Ok(result)
}

pub(crate) fn originating_keys() -> BTreeSet<String> {
    let mut keys = process_state().lock().unwrap_or_else(|error| error.into_inner()).originating_keys.clone();
    keys.extend(retained_keys(std::env::var(RETAIN).ok().as_deref()));
    keys
}
fn retained_keys(raw: Option<&str>) -> BTreeSet<String> {
    raw.filter(|raw| raw.len() <= 65536).and_then(|raw| serde_json::from_str::<BTreeSet<String>>(raw).ok()).unwrap_or_default().into_iter().filter(|key| !key.is_empty() && !key.contains(['=', '\0']) && !SELECTORS.contains(&key.as_str())).collect()
}
pub(crate) fn receipt() -> Option<EnvironmentReceipt> {
    process_state().lock().unwrap_or_else(|error| error.into_inner()).receipt.clone()
}
pub(crate) fn diagnostic_json() -> serde_json::Value {
    serde_json::to_value(receipt()).unwrap_or(serde_json::Value::Null)
}
pub(crate) fn diagnostic_text() -> String {
    let Some(receipt) = receipt() else {
        return "No environment-file resolution for this invocation.".into();
    };
    let mut lines = vec![
        format!("Project: {}", receipt.project.display()),
        format!("Selector scope: {}", receipt.selector_scope.display()),
        format!("Scope: {}", receipt.scope.display()),
        format!("Override: {} [{:?}]", receipt.override_values, receipt.override_source),
    ];
    for (i, source) in receipt.sources.iter().enumerate() {
        lines.push(format!("{}. {} [loaded; {} variables]", i + 1, source.path.display(), source.variables));
    }
    for (key, provenance) in receipt.variables {
        lines.push(format!("  {key}: {provenance:?}"));
    }
    if !receipt.withheld_keys.is_empty() {
        lines.push(format!("Withheld selectors: {}", receipt.withheld_keys.into_iter().collect::<Vec<_>>().join(", ")));
    }
    if receipt.sources.is_empty() {
        lines.push("Sources: none".into());
    }
    lines.join("\n")
}

/// Reconstruct from the current invocation, never from a daemon/global map.
/// Keys include inherited winners of a selected file variable as well as file
/// winners, so the caller-wins rule survives shell cleanup too.
pub(crate) fn restore_originating_values(target: &mut HashMap<String, String>, originating: &HashMap<String, String>, keys: &BTreeSet<String>) {
    for key in keys {
        if let Some(value) = originating.get(key) {
            insert_value(target, key.clone(), value.clone());
        }
    }
    if !keys.is_empty()
        && let Ok(retained) = serde_json::to_string(keys)
    {
        target.insert(RETAIN.into(), retained);
    }
}

fn contains_key(values: &HashMap<String, String>, key: &str) -> bool {
    if cfg!(windows) { values.keys().any(|existing| existing.eq_ignore_ascii_case(key)) } else { values.contains_key(key) }
}
fn insert_value(values: &mut HashMap<String, String>, key: String, value: String) {
    if cfg!(windows) {
        values.retain(|existing, _| !existing.eq_ignore_ascii_case(&key));
    }
    values.insert(key, value);
}
fn parse_dotenv(path: &Path) -> Result<Vec<(String, String)>> {
    if path.file_name().is_some_and(|name| name == ".envrc") {
        bail!(".envrc requires authorized direnv and cannot be loaded as dotenv: {}", path.display());
    }
    let file = std::fs::File::open(path).map_err(|error| anyhow::anyhow!("cannot read environment file {} ({:?})", path.display(), error.kind()))?;
    let mut entries = Vec::new();
    for entry in dotenvy::from_read_iter(file) {
        let (key, value) = entry.map_err(|error| match error {
            dotenvy::Error::Io(error) => anyhow::anyhow!("cannot read environment file {} ({:?})", path.display(), error.kind()),
            _ => anyhow::anyhow!("invalid dotenv syntax in {}", path.display()),
        })?;
        if key.contains('\0') || value.contains('\0') {
            bail!("environment file contains a NUL byte: {}", path.display());
        }
        entries.push((key, value));
    }
    Ok(entries)
}
fn selected_paths(args: &[String], env: &HashMap<String, String>, cwd: &Path) -> Result<(PathBuf, Option<PathBuf>)> {
    let action = crate::cli::router::resolve_action(args);
    let launch = matches!(&action, crate::cli::router::Action::Launch { .. });
    let work = matches!(&action, crate::cli::router::Action::Command { cmd, .. } if matches!(cmd.as_str(), "workbench" | "work"));
    let scoped = matches!(&action, crate::cli::router::Action::Command { cmd, .. } if matches!(cmd.as_str(), "daemon" | "observe" | "observe-web" | "workbench" | "work"));
    let mut project = None;
    let mut scope = None;
    let mut remote = false;
    let mut i = 0;
    while i < args.len() && args[i] != "--" {
        let arg = args[i].as_str();
        if arg == "--device" || arg.starts_with("--device=") {
            remote = true;
        }
        if (launch && arg == "--dir") || (work && arg == "--project") {
            project = Some(args.get(i + 1).context("project option requires a path")?.clone());
        } else if launch && arg.starts_with("--dir=") {
            project = Some(arg[6..].to_string());
        } else if work && arg.starts_with("--project=") {
            project = Some(arg[10..].to_string());
        } else if scoped && arg == "--scope" {
            scope = Some(args.get(i + 1).context("--scope requires a path")?.clone());
        } else if scoped && let Some(value) = arg.strip_prefix("--scope=") {
            scope = Some(value.to_string());
        }
        i += if VALUE_FLAGS.contains(&arg) { 2 } else { 1 };
    }
    // Codex stdio has already pinned its canonical project in argv. Other
    // launches use only their own --dir, never a daemon or unrelated app hint.
    if remote {
        project = None;
    }
    let project = project.map(|raw| expand_path(cwd, &raw, env)).transpose()?.unwrap_or_else(|| cwd.to_path_buf());
    if !project.is_dir() {
        bail!("selected local project is not a directory: {}", project.display());
    }
    let scope = scope.map(|raw| expand_path(cwd, &raw, env)).transpose()?;
    Ok((project.canonicalize().unwrap_or(project), scope))
}
fn expand_path(base: &Path, raw: &str, env: &HashMap<String, String>) -> Result<PathBuf> {
    let path = if raw == "~" || raw.starts_with("~/") || raw.starts_with("~\\") {
        let home = env.get("HOME").or_else(|| env.get("USERPROFILE")).filter(|value| !value.is_empty()).context("home-expanded environment path requires HOME or USERPROFILE")?;
        PathBuf::from(home).join(raw.get(2..).unwrap_or(""))
    } else {
        PathBuf::from(raw)
    };
    let absolute = if path.is_absolute() { path } else { base.join(path) };
    Ok(absolute.canonicalize().unwrap_or(absolute))
}
fn split_file_list(value: &str) -> Vec<String> {
    value.split(',').map(str::trim).filter(|value| !value.is_empty()).map(str::to_string).collect()
}
pub(crate) fn parse_bool(value: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" | "" => Ok(false),
        _ => bail!("AGENTX_ENV_OVERRIDE must be true or false"),
    }
}

#[cfg(test)]
#[path = "environment/tests.rs"]
mod tests;
