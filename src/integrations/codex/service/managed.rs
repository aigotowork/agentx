//! Acquires one officially managed native daemon per canonical Codex home.
//! Ownership evidence lives in AgentX storage, never in user config. Native
//! start installs its own managed package on first use; no stop/restart path
//! exists here. See the service case model in `topology.md`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::transport::UnixConnection;
use super::{CodexService, ManagedCodexService, SHARED_NATIVE_VERSIONS, ServiceCompatibility, ServiceConnection, ServiceFailure, ServiceIdentity, ServiceRequest, client_variable, supports_shared_native};
use crate::agents::attachments::ExecutionId;

#[cfg(test)]
const VERIFIED_VERSION: &str = "0.167.0";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

/// This is retained acquisition evidence, not a second service registry. Its
/// generation must match the current native PID record and live OS incarnation.
#[derive(Serialize, Deserialize)]
struct Receipt {
    schema: u32,
    home: PathBuf,
    execution: String,
    process_identity: String,
    version: String,
    program: PathBuf,
    environment: String,
    #[serde(default)]
    environment_keys: BTreeMap<String, String>,
    configuration: String,
    /// New readers can ignore a verified terminal-only preference. Keep the
    /// original digest readable by older clients for the exact configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    server_configuration: Option<String>,
    agentx_dir: PathBuf,
}

struct LiveDaemon {
    pid: u32,
    execution: ExecutionId,
    process_identity: String,
}

struct ConfigurationInputs {
    fingerprint: String,
    legacy_fingerprint: String,
    environment_dependencies: BTreeSet<String>,
}

impl CodexService for ManagedCodexService {
    fn connect(&mut self, request: &ServiceRequest) -> Result<ServiceConnection, ServiceFailure> {
        validate_invocation(&request.args)?;
        let home = request.canonical_home.canonicalize().map_err(unavailable)?;
        if home != request.canonical_home || !home.is_dir() {
            return Err(ServiceFailure::UnsupportedInvocation("canonical_home must name the existing canonical Codex home".into()));
        }
        let program = request.program.canonicalize().map_err(unavailable)?;
        let agentx_dir = request.agentx_dir.canonicalize().map_err(unavailable)?;
        let configuration = configuration_inputs(&home, &request.prefix_args)?;
        if let Some(key) = configuration.environment_dependencies.iter().find(|key| key.starts_with("AGENTX_") && client_variable(key)) {
            return Err(ServiceFailure::Incompatible(format!("startup configuration references client-owned environment variable {key}; a shared service cannot supply one participant's identity or settings")));
        }
        let env = backend_environment(&request.env, &home, &agentx_dir, &configuration.environment_dependencies);
        let native_version = native_version(request, &env, &home)?;
        if let Some(expected) = request.required_version
            && native_version != expected
        {
            return Err(ServiceFailure::Incompatible(format!("direct terminal requires Codex {expected}; found {native_version}")));
        }
        if !supports_shared_native(&native_version) {
            return Err(ServiceFailure::Incompatible(format!("native version {native_version}; supported managed service versions: {}", SHARED_NATIVE_VERSIONS.join(", "))));
        }
        let compatibility = ServiceCompatibility { native_program: program, environment_fingerprint: fingerprint(&env), configuration_fingerprint: configuration.fingerprint, agentx_dir: agentx_dir.clone() };
        let environment_keys = environment_key_digests(&env);
        let state = agentx_dir.join("codex-services").join(digest(home.as_os_str().as_encoded_bytes()));
        fs::create_dir_all(&state).map_err(unavailable)?;
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).map_err(unavailable)?;
        let lock = OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600).custom_flags(libc::O_NOFOLLOW).open(state.join("acquire.lock")).map_err(unavailable)?;
        crate::sys::fs::lock_exclusive(&lock).map_err(unavailable)?;
        let receipt_path = state.join("service.json");
        let mut receipt = read_receipt(&receipt_path)?;
        let socket = home.join("app-server-control/app-server-control.sock");
        let live = observe_daemon(&home)?;
        let (daemon, transport) = if let Some(daemon) = live {
            let known =
                receipt.as_mut().ok_or_else(|| ServiceFailure::Incompatible("an existing native daemon has no AgentX compatibility receipt; leave it running and select a separate Codex home or explicitly arrange compatible ownership".into()))?;
            validate_receipt(known, &home, &daemon, &native_version, &compatibility, &environment_keys, &configuration.legacy_fingerprint)?;
            let transport = UnixConnection::connect(&socket)?;
            (daemon, transport)
        } else {
            if socket.exists() && receipt.is_none() {
                return Err(ServiceFailure::Incompatible(format!("unowned native control socket at {}; it will not be replaced", socket.display())));
            }
            let execution = format!("codex:{}", uuid::Uuid::new_v4());
            let mut service_env = env.clone();
            service_env.insert("AGENTX_PROCESS_ID".into(), execution.clone());
            service_env.insert("AGENTX_CODEX_SHARED".into(), "1".into());
            let value = native_json(request, &service_env, &home, &["app-server", "daemon", "start"])?;
            if value.get("status").and_then(Value::as_str) != Some("started") {
                return Err(ServiceFailure::Incompatible("another owner started the native daemon during acquisition; its configuration has not been accepted".into()));
            }
            verify_start_report(&value, &socket, &native_version)?;
            let daemon = observe_daemon(&home)?.ok_or_else(|| ServiceFailure::Unavailable("daemon start returned without a verifiable live backend generation".into()))?;
            if daemon.execution.0 != execution {
                return Err(ServiceFailure::Incompatible("native backend did not retain the selected service generation".into()));
            }
            let transport = UnixConnection::connect(&socket)?;
            let started_receipt = Receipt {
                schema: 1,
                home: home.clone(),
                execution: daemon.execution.0.clone(),
                process_identity: daemon.process_identity.clone(),
                version: native_version.clone(),
                program: compatibility.native_program.clone(),
                environment: compatibility.environment_fingerprint.clone(),
                environment_keys,
                configuration: configuration.legacy_fingerprint.clone(),
                server_configuration: Some(compatibility.configuration_fingerprint.clone()),
                agentx_dir,
            };
            write_receipt(&state, &receipt_path, &started_receipt)?;
            receipt = Some(started_receipt);
            (daemon, transport)
        };
        // A changed process or socket during the handshake must not attach a
        // connection to the previous execution's identity or recovery evidence.
        let after = observe_daemon(&home)?.ok_or_else(|| ServiceFailure::Unavailable("native backend exited while connecting".into()))?;
        if after.execution != daemon.execution || after.process_identity != daemon.process_identity || transport.peer_pid()? != daemon.pid {
            return Err(ServiceFailure::Incompatible("native backend generation changed while connecting".into()));
        }
        if configuration_inputs(&home, &request.prefix_args)?.fingerprint != compatibility.configuration_fingerprint {
            return Err(ServiceFailure::Incompatible("native startup configuration changed while connecting".into()));
        }
        if let Some(receipt) = &mut receipt
            && receipt.server_configuration.is_none()
        {
            receipt.server_configuration = Some(compatibility.configuration_fingerprint.clone());
            write_receipt(&state, &receipt_path, receipt)?;
        }
        Ok(ServiceConnection { identity: ServiceIdentity { canonical_home: home, execution: daemon.execution, native_version, control_socket: socket, compatibility }, transport: Box::new(transport) })
    }
}

/// CLI overlays are applied and verified by the thread composition boundary;
/// native managed start discards them. Reject anything that would be ignored.
fn validate_invocation(args: &[String]) -> Result<(), ServiceFailure> {
    let mut found = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "app-server" if !found => found = true,
            "--stdio" => {}
            "--listen" if args.get(index + 1).is_some_and(|arg| arg == "stdio://") => index += 1,
            "--listen=stdio://" => {}
            other => return Err(ServiceFailure::UnsupportedInvocation(format!("{other}: only the stdio app-server entry can acquire a shared daemon; configuration overrides must be applied as verified thread configuration first"))),
        }
        index += 1;
    }
    if !found {
        return Err(ServiceFailure::UnsupportedInvocation("expected app-server".into()));
    }
    Ok(())
}

/// Strip client identity and environment provenance before it becomes the
/// baseline inherited by every native thread's tools and hooks.
fn backend_environment(env: &HashMap<String, String>, home: &Path, agentx_dir: &Path, dependencies: &BTreeSet<String>) -> BTreeMap<String, String> {
    let mut result: BTreeMap<_, _> = env.iter().filter(|(key, _)| dependencies.contains(*key) || !client_variable(key)).map(|(key, value)| (key.clone(), value.clone())).collect();
    // The native shell fills LC_CTYPE from its effective locale. Retain an
    // explicit differing locale; only erase the redundant derived value.
    let locale = env.get("LC_ALL").filter(|value| !value.is_empty()).or_else(|| env.get("LANG"));
    if !dependencies.contains("LC_CTYPE") && env.get("LC_CTYPE").is_some_and(|value| Some(value) == locale) {
        result.remove("LC_CTYPE");
    }
    if !dependencies.contains("PATH")
        && let Some(path) = result.get_mut("PATH")
    {
        *path = normalize_runner_path(path, home);
    }
    result.insert("CODEX_HOME".into(), home.to_string_lossy().into_owned());
    result.insert("AGENTX_DIR".into(), agentx_dir.to_string_lossy().into_owned());
    result
}

/// Codex prepends its native helper directory; a nested terminal runner can
/// then prepend its own tool dirs ahead of it. Keep actual user search order
/// and strip only the known package directory and repeated entries.
fn normalize_runner_path(path: &str, home: &Path) -> String {
    let helper = home.join("packages/app-server-daemon/current").canonicalize().ok().map(|package| package.join("codex-path"));
    let mut seen = BTreeSet::new();
    let paths = std::env::split_paths(path).filter(|entry| Some(entry) != helper.as_ref() && seen.insert(entry.clone()));
    std::env::join_paths(paths).map(|value| value.to_string_lossy().into_owned()).unwrap_or_else(|_| path.to_owned())
}

fn native_version(request: &ServiceRequest, env: &BTreeMap<String, String>, home: &Path) -> Result<String, ServiceFailure> {
    let output = native_output(request, env, home, &["--version"])?;
    output.trim().strip_prefix("codex-cli ").filter(|version| !version.is_empty() && !version.contains(char::is_whitespace)).map(str::to_owned).ok_or_else(|| ServiceFailure::Protocol("native --version did not return codex-cli <version>".into()))
}

fn native_json(request: &ServiceRequest, env: &BTreeMap<String, String>, home: &Path, args: &[&str]) -> Result<Value, ServiceFailure> {
    serde_json::from_str(&native_output(request, env, home, args)?).map_err(|error| ServiceFailure::Protocol(format!("native daemon report: {error}")))
}

/// File-backed diagnostics avoid an inherited pipe keeping acquisition alive
/// after the daemon has detached, and bound what is loaded into memory.
fn native_output(request: &ServiceRequest, env: &BTreeMap<String, String>, home: &Path, args: &[&str]) -> Result<String, ServiceFailure> {
    let mut stdout = tempfile::tempfile().map_err(unavailable)?;
    let mut stderr = tempfile::tempfile().map_err(unavailable)?;
    let mut child = Command::new(&request.program)
        .args(&request.prefix_args)
        .args(args)
        .env_clear()
        .envs(env)
        .current_dir(home)
        .stdin(Stdio::null())
        .stdout(stdout.try_clone().map_err(unavailable)?)
        .stderr(stderr.try_clone().map_err(unavailable)?)
        .spawn()
        .map_err(unavailable)?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait().map_err(unavailable)? {
            Some(status) => break status,
            None if started.elapsed() >= COMMAND_TIMEOUT => {
                // Only the acquisition CLI child is owned, never a daemon PID
                // or process group. A timed-out start is an unknown outcome.
                let _ = child.kill();
                let _ = child.wait();
                return Err(ServiceFailure::Unavailable("native acquisition timed out; daemon outcome is unknown and will not be restarted".into()));
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let out = read_output(&mut stdout)?;
    if !status.success() {
        let diagnostic = read_output(&mut stderr)?;
        // Do not include argv/env/config values, which may contain secrets.
        return Err(ServiceFailure::Unavailable(format!("native {} failed ({status}): {}", args.join(" "), diagnostic.trim())));
    }
    Ok(out)
}

fn read_output(file: &mut File) -> Result<String, ServiceFailure> {
    file.seek(SeekFrom::Start(0)).map_err(unavailable)?;
    let mut output = String::new();
    file.take(1_048_577).read_to_string(&mut output).map_err(unavailable)?;
    if output.len() > 1_048_576 {
        return Err(ServiceFailure::Protocol("native management output exceeded 1 MiB".into()));
    }
    Ok(output)
}

fn observe_daemon(home: &Path) -> Result<Option<LiveDaemon>, ServiceFailure> {
    let path = home.join("app-server-daemon/daemon.pid");
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(unavailable(error)),
    };
    let record: Value = serde_json::from_slice(&bytes).map_err(|_| ServiceFailure::Incompatible("native daemon PID record is malformed; it will not be replaced".into()))?;
    let pid = record.get("pid").and_then(Value::as_u64).and_then(|pid| u32::try_from(pid).ok()).filter(|pid| *pid > 0 && *pid <= i32::MAX as u32).ok_or_else(|| ServiceFailure::Protocol("invalid native daemon PID".into()))?;
    let Some(observed) = crate::sys::process::identity(pid) else {
        if crate::sys::process::is_alive(pid) {
            return Err(ServiceFailure::Unavailable("cannot verify the live native backend incarnation".into()));
        }
        return Ok(None);
    };
    let identity = record.get("processIdentity").ok_or_else(|| ServiceFailure::Incompatible("native PID record has no supported process identity".into()))?;
    let boot = identity.get("bootId").and_then(Value::as_str).ok_or_else(|| ServiceFailure::Protocol("native PID record lacks bootId".into()))?;
    let ticks = identity.get("startTicks").and_then(Value::as_u64).ok_or_else(|| ServiceFailure::Protocol("native PID record lacks startTicks".into()))?;
    if observed != format!("linux:{boot}:{ticks}") {
        return Err(ServiceFailure::Incompatible("native PID record points to a different process incarnation; no service action was taken".into()));
    }
    let env = fs::read(format!("/proc/{pid}/environ")).map_err(unavailable)?;
    let variable = |key: &str| env.split(|byte| *byte == 0).find_map(|entry| entry.strip_prefix(key.as_bytes()).and_then(|value| value.strip_prefix(b"=")).and_then(|value| std::str::from_utf8(value).ok()));
    let token = variable("AGENTX_PROCESS_ID").filter(|token| token.starts_with("codex:") && token.len() > 6).ok_or_else(|| ServiceFailure::Incompatible("existing daemon has no AgentX-owned backend generation; it was left unchanged".into()))?;
    if variable("AGENTX_CODEX_SHARED") != Some("1") || variable("AGENTX_INSTANCE_NAME").is_some() {
        return Err(ServiceFailure::Incompatible("existing daemon has incompatible shared identity context; it was left unchanged".into()));
    }
    Ok(Some(LiveDaemon { pid, execution: ExecutionId(token.to_owned()), process_identity: observed }))
}

fn verify_start_report(report: &Value, socket: &Path, version: &str) -> Result<(), ServiceFailure> {
    if report.get("backend").and_then(Value::as_str) != Some("pid") || report.get("socketPath").and_then(Value::as_str) != socket.to_str() || report.get("appServerVersion").and_then(Value::as_str) != Some(version) {
        return Err(ServiceFailure::Incompatible("native start selected an unverified backend, socket or version; it was left running for inspection".into()));
    }
    Ok(())
}

fn configuration_inputs(home: &Path, prefix: &[String]) -> Result<ConfigurationInputs, ServiceFailure> {
    let mut entries = Vec::new();
    let mut legacy = Vec::new();
    let mut environment_dependencies = BTreeSet::new();
    for path in [home.join("config.toml"), home.join("requirements.toml"), PathBuf::from("/etc/codex/config.toml"), PathBuf::from("/etc/codex/requirements.toml")] {
        let (content, raw) = match fs::read(&path) {
            Ok(content) => {
                let mut value: toml::Value = std::str::from_utf8(&content).ok().and_then(|text| toml::from_str(text).ok()).ok_or_else(|| ServiceFailure::Protocol(format!("cannot parse native startup configuration at {}", path.display())))?;
                collect_environment_dependencies(&value, &mut environment_dependencies);
                let raw = digest(&content);
                let semantic = if path == home.join("config.toml") {
                    // Stock TUI writes this local accessibility receipt after
                    // startup. No model, permissions or hook setting is ignored.
                    if let Some(table) = value.as_table_mut()
                        && let Some(tui) = table.get_mut("tui").and_then(toml::Value::as_table_mut)
                    {
                        tui.remove("screen_reader_detection_done");
                        if tui.is_empty() {
                            table.remove("tui");
                        }
                    }
                    fingerprint(&value)
                } else {
                    raw.clone()
                };
                (Some(semantic), Some(raw))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (None, None),
            Err(error) => return Err(unavailable(error)),
        };
        legacy.push((path.clone(), raw));
        entries.push((path, content));
    }
    Ok(ConfigurationInputs { fingerprint: fingerprint(&(entries, prefix)), legacy_fingerprint: fingerprint(&(legacy, prefix)), environment_dependencies })
}

/// These explicit native references prevent normalization of a marker-looking
/// credential/MCP input, including when it was absent at daemon startup. This
/// is an opt-out guard, not a complete dependency projection: unknown variables
/// still participate in compatibility and new additions remain incompatible.
fn collect_environment_dependencies(value: &toml::Value, dependencies: &mut BTreeSet<String>) {
    match value {
        toml::Value::Table(table) => {
            for (key, value) in table {
                match (key.as_str(), value) {
                    ("env_key" | "bearer_token_env_var", toml::Value::String(name)) => {
                        dependencies.insert(name.clone());
                    }
                    ("env_vars", toml::Value::Array(names)) => {
                        dependencies.extend(names.iter().filter_map(toml::Value::as_str).map(str::to_owned));
                    }
                    ("env_http_headers", toml::Value::Table(headers)) => {
                        dependencies.extend(headers.values().filter_map(toml::Value::as_str).map(str::to_owned));
                    }
                    ("env", toml::Value::Table(environment)) => {
                        dependencies.extend(environment.keys().cloned());
                    }
                    _ => {}
                }
                collect_environment_dependencies(value, dependencies);
            }
        }
        toml::Value::Array(values) => {
            for value in values {
                collect_environment_dependencies(value, dependencies);
            }
        }
        _ => {}
    }
}

fn read_receipt(path: &Path) -> Result<Option<Receipt>, ServiceFailure> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|_| ServiceFailure::Incompatible("AgentX service compatibility receipt is malformed".into())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(unavailable(error)),
    }
}

fn validate_receipt(receipt: &Receipt, home: &Path, daemon: &LiveDaemon, version: &str, compatibility: &ServiceCompatibility, environment_keys: &BTreeMap<String, String>, legacy_configuration: &str) -> Result<(), ServiceFailure> {
    let mut differences = Vec::new();
    for (same, field) in [
        (receipt.schema == 1, "receipt schema"),
        (receipt.home == home, "canonical home"),
        (receipt.execution == daemon.execution.0, "execution token"),
        (receipt.process_identity == daemon.process_identity, "native process incarnation"),
        (receipt.version == version, "native version"),
        (receipt.program == compatibility.native_program, "native program"),
        (receipt.server_configuration.as_deref().map_or(receipt.configuration == legacy_configuration, |expected| expected == compatibility.configuration_fingerprint), "startup configuration"),
        (receipt.agentx_dir == compatibility.agentx_dir, "AgentX storage"),
    ] {
        if !same {
            differences.push(field.to_owned());
        }
    }
    if receipt.environment != compatibility.environment_fingerprint {
        let keys = if receipt.environment_keys.is_empty() { Vec::new() } else { changed_environment_keys(&receipt.environment_keys, environment_keys) };
        differences.push(if keys.is_empty() { "backend environment".into() } else { format!("backend environment keys [{}]", keys.join(", ")) });
    }
    if !differences.is_empty() {
        return Err(ServiceFailure::Incompatible(format!("{} differ from the accepted service receipt; the running backend will not be restarted or replaced", differences.join("; "))));
    }
    Ok(())
}

fn environment_key_digests(environment: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    environment.iter().map(|(key, value)| (key.clone(), digest(value.as_bytes()))).collect()
}

fn changed_environment_keys(previous: &BTreeMap<String, String>, current: &BTreeMap<String, String>) -> Vec<String> {
    previous.keys().chain(current.keys()).collect::<BTreeSet<_>>().into_iter().filter(|key| previous.get(*key) != current.get(*key)).take(16).cloned().collect()
}

fn write_receipt(state: &Path, path: &Path, receipt: &Receipt) -> Result<(), ServiceFailure> {
    use std::io::Write;
    let mut pending = tempfile::NamedTempFile::new_in(state).map_err(unavailable)?;
    serde_json::to_writer(pending.as_file_mut(), receipt).map_err(|error| ServiceFailure::Protocol(error.to_string()))?;
    pending.as_file_mut().flush().map_err(unavailable)?;
    pending.as_file().sync_all().map_err(unavailable)?;
    pending.persist(path).map_err(|error| unavailable(error.error))?;
    Ok(())
}

fn fingerprint(value: &impl Serialize) -> String {
    digest(&serde_json::to_vec(value).expect("service fingerprint inputs are JSON-serializable"))
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unavailable(error: std::io::Error) -> ServiceFailure {
    ServiceFailure::Unavailable(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_identity_and_cwd_do_not_partition_backend_environment() {
        let home = Path::new("/codex");
        let storage = Path::new("/agentx");
        let mut first = HashMap::from([("AGENTX_PROCESS_ID".into(), "first".into()), ("AGENTX_INSTANCE_NAME".into(), "sake".into()), ("PASEO_AGENT_CWD".into(), "/one".into()), ("PWD".into(), "/one".into()), ("PATH".into(), "/bin".into())]);
        let before = backend_environment(&first, home, storage, &BTreeSet::new());
        first.insert("AGENTX_PROCESS_ID".into(), "second".into());
        first.insert("AGENTX_INSTANCE_NAME".into(), "kana".into());
        first.insert("PWD".into(), "/two".into());
        first.insert("PASEO_AGENT_CWD".into(), "/two".into());
        assert_eq!(before, backend_environment(&first, home, storage, &BTreeSet::new()));
        assert!(!before.contains_key("AGENTX_PROCESS_ID"));
        first.insert("PATH".into(), "/different/bin".into());
        assert_ne!(before, backend_environment(&first, home, storage, &BTreeSet::new()));
    }

    #[test]
    fn ignored_startup_flags_are_rejected_before_service_management() {
        for args in [vec!["-c", "features.hooks=true", "app-server"], vec!["app-server", "--startup-sign-out"], vec!["app-server", "--listen", "unix:///private.sock"], vec!["app-server", "daemon", "restart"]] {
            assert!(matches!(validate_invocation(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()), Err(ServiceFailure::UnsupportedInvocation(_))));
        }
        assert!(validate_invocation(&["app-server".into(), "--listen=stdio://".into()]).is_ok());
    }

    #[test]
    fn native_tool_environment_reuses_baseline_but_keeps_material_conflicts() {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("packages/app-server-daemon/current");
        fs::create_dir_all(package.join("codex-path")).unwrap();
        let storage = root.path().join("agentx");
        let base = HashMap::from([
            ("PATH".into(), "/custom/bin:/usr/bin:/bin".into()),
            ("LANG".into(), "C.UTF-8".into()),
            ("LC_ALL".into(), "C.UTF-8".into()),
            ("TERM".into(), "xterm-256color".into()),
            ("OPENAI_API_KEY".into(), "fixture-credential".into()),
            ("HTTP_PROXY".into(), "http://fixture.invalid".into()),
        ]);
        let mut shell = base.clone();
        shell.insert("PATH".into(), format!("/custom/bin:{}:/custom/bin:/usr/bin:/bin", package.canonicalize().unwrap().join("codex-path").display()));
        for (key, value) in [
            ("CODEX_SESSION_ID", "root"),
            ("CODEX_THREAD_ID", "root"),
            ("CODEX_TOOL_CALL_ID", "call"),
            ("CODEX_VERSION", VERIFIED_VERSION),
            ("CODEX_CI", "1"),
            ("CODEX_APPLY_PATCH_PRESERVE_LINE_ENDINGS", "1"),
            ("TERM", "dumb"),
            ("COLORTERM", ""),
            ("NO_COLOR", "1"),
            ("PAGER", "cat"),
            ("GIT_PAGER", "cat"),
            ("GH_PAGER", "cat"),
            ("LOGNAME", "fixture-user"),
            ("LC_CTYPE", "C.UTF-8"),
        ] {
            shell.insert(key.into(), value.into());
        }
        let expected = backend_environment(&base, root.path(), &storage, &BTreeSet::new());
        assert_eq!(expected, backend_environment(&shell, root.path(), &storage, &BTreeSet::new()));
        let path_dependency = BTreeSet::from(["PATH".to_owned()]);
        assert_ne!(backend_environment(&base, root.path(), &storage, &path_dependency), backend_environment(&shell, root.path(), &storage, &path_dependency));
        for (key, value) in [("PATH", "/other/bin:/usr/bin:/bin"), ("OPENAI_API_KEY", "different-credential"), ("HTTP_PROXY", "http://other.invalid"), ("LC_CTYPE", "different-locale"), ("UNRELATED_THREAD_VALUE", "new")] {
            let mut changed = shell.clone();
            changed.insert(key.into(), value.into());
            assert_ne!(expected, backend_environment(&changed, root.path(), &storage, &BTreeSet::new()), "lost material conflict for {key}");
        }
    }

    #[test]
    fn environment_conflict_diagnostics_identify_keys_without_values() {
        let before = BTreeMap::from([("PATH".into(), "original-path".into()), ("OPENAI_API_KEY".into(), "first-credential".into())]);
        let after = BTreeMap::from([("PATH".into(), "original-path".into()), ("OPENAI_API_KEY".into(), "second-credential".into()), ("EXTRA".into(), "private-value".into())]);
        let retained = environment_key_digests(&before);
        assert_eq!(changed_environment_keys(&retained, &environment_key_digests(&after)), vec!["EXTRA", "OPENAI_API_KEY"]);
        let serialized = serde_json::to_string(&retained).unwrap();
        assert!(!serialized.contains("first-credential"));
        assert!(!serialized.contains("original-path"));
    }

    #[test]
    fn configured_environment_dependencies_opt_out_of_native_normalization() {
        let config: toml::Value = toml::from_str(
            r#"
[model_providers.fixture]
env_key = "CODEX_CI"
env_http_headers = { "X-Terminal" = "TERM", "X-Locale" = "LC_CTYPE" }
[mcp_servers.fixture]
command = "fixture"
bearer_token_env_var = "CODEX_TOOL_CALL_ID"
env_vars = ["PAGER", "LOGNAME"]
env = { GH_PAGER = "fixed" }
"#,
        )
        .unwrap();
        let mut dependencies = BTreeSet::new();
        collect_environment_dependencies(&config, &mut dependencies);
        assert_eq!(dependencies, BTreeSet::from(["CODEX_CI", "TERM", "LC_CTYPE", "PAGER", "LOGNAME", "GH_PAGER", "CODEX_TOOL_CALL_ID"].map(str::to_owned)));
        let home = Path::new("/fixture-codex");
        let storage = Path::new("/fixture-agentx");
        let base = HashMap::from([("TERM".into(), "xterm".into()), ("LC_ALL".into(), "C.UTF-8".into())]);
        let expected = backend_environment(&base, home, storage, &dependencies);
        for (key, value) in [("CODEX_CI", "1"), ("TERM", "dumb"), ("LC_CTYPE", "C.UTF-8"), ("PAGER", "cat"), ("LOGNAME", "fixture-user"), ("GH_PAGER", "cat"), ("CODEX_TOOL_CALL_ID", "fixture-token")] {
            let mut child = base.clone();
            child.insert(key.into(), value.into());
            assert_ne!(expected, backend_environment(&child, home, storage, &dependencies), "ignored declared dependency {key}");
        }
    }

    #[test]
    fn receipt_fences_new_generation_and_changed_configuration() {
        let home = Path::new("/codex");
        let generation = ExecutionId("old-run".into());
        let mut daemon = LiveDaemon { pid: 1, execution: generation.clone(), process_identity: "incarnation".into() };
        let mut compatibility = ServiceCompatibility { native_program: "/native".into(), environment_fingerprint: "env".into(), configuration_fingerprint: "config".into(), agentx_dir: "/agentx".into() };
        let environment_keys = BTreeMap::new();
        let receipt = Receipt {
            schema: 1,
            home: home.into(),
            execution: generation.0.clone(),
            process_identity: "incarnation".into(),
            version: VERIFIED_VERSION.into(),
            program: "/native".into(),
            environment: "env".into(),
            environment_keys: environment_keys.clone(),
            configuration: "raw".into(),
            server_configuration: Some("config".into()),
            agentx_dir: "/agentx".into(),
        };
        assert!(validate_receipt(&receipt, home, &daemon, VERIFIED_VERSION, &compatibility, &environment_keys, "raw").is_ok());
        daemon.execution = ExecutionId("new-run".into());
        assert!(validate_receipt(&receipt, home, &daemon, VERIFIED_VERSION, &compatibility, &environment_keys, "raw").is_err());
        daemon.execution = generation;
        daemon.process_identity = "reused-pid".into();
        assert!(validate_receipt(&receipt, home, &daemon, VERIFIED_VERSION, &compatibility, &environment_keys, "raw").is_err());
        daemon.process_identity = "incarnation".into();
        compatibility.configuration_fingerprint = "changed".into();
        assert!(validate_receipt(&receipt, home, &daemon, VERIFIED_VERSION, &compatibility, &environment_keys, "raw").is_err());
    }
}
