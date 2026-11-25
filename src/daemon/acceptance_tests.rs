//! Independent process acceptance. Native wire types remain authoritative;
//! cases use an explicitly selected source-built binary in disposable scopes.
//! See `.docs/architecture/acceptance.md` DD-01/03/04/08/11.

#![cfg(target_os = "linux")]

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::wire::{BUILD_VERSION, ClientFrame, PROTOCOL_VERSION, ServerFrame};
use super::{DaemonError, DaemonInfo, DaemonScope};
use crate::service::{CallerContext, InvocationContext, Request, RequestId, Response};

/// Owns only disposable filesystem state and processes whose executable and
/// exact daemon scope can be checked before cleanup. No inherited identity,
/// credential, dev-root re-exec or default config is admitted to child env.
struct Fixture {
    root: tempfile::TempDir,
    binary: PathBuf,
    build_id: String,
    artifact_sha256: String,
    sequence: AtomicUsize,
}

struct OwnedDaemon(Child);

impl std::ops::Deref for OwnedDaemon {
    type Target = Child;
    fn deref(&self) -> &Child {
        &self.0
    }
}

impl std::ops::DerefMut for OwnedDaemon {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.0
    }
}

impl Drop for OwnedDaemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Fixture {
    fn new() -> Self {
        let binary = fs::canonicalize(std::env::var_os("AGENTX_ACCEPTANCE_BINARY").expect("set AGENTX_ACCEPTANCE_BINARY to the fixed candidate executable")).unwrap();
        let root = tempfile::Builder::new().prefix("agentx-ddd-process-").tempdir().unwrap();
        for directory in ["home", "workspace", "tmp", "scope", "second"] {
            fs::create_dir(root.path().join(directory)).unwrap();
        }
        let image = fs::read(&binary).unwrap();
        let build_id = artifact_cohort(&image);
        let artifact_sha256: String = Sha256::digest(&image).iter().map(|byte| format!("{byte:02x}")).collect();
        println!("fixed product={} cohort={build_id} artifact_sha256={artifact_sha256}", binary.display());
        Self { root, binary, build_id, artifact_sha256, sequence: AtomicUsize::new(0) }
    }

    fn scope(&self, leaf: &str) -> DaemonScope {
        DaemonScope::prepare(self.root.path().join(leaf)).unwrap()
    }

    fn command(&self, scope: &Path) -> Command {
        let home = self.root.path().join("home");
        let mut command = Command::new(&self.binary);
        command
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LANG", "C.UTF-8")
            .env("CI", "1")
            .env("NO_COLOR", "1")
            .env("AGENTX_UPDATE_VERIFY", "1")
            .env("HOME", &home)
            .env("AGENTX_DIR", scope)
            .env("TMPDIR", self.root.path().join("tmp"))
            .env("XDG_CONFIG_HOME", home.join("config"))
            .env("XDG_CACHE_HOME", home.join("cache"))
            .env("XDG_DATA_HOME", home.join("data"))
            .env("CODEX_HOME", home.join("codex"))
            .env("CLAUDE_CONFIG_DIR", home.join("claude"))
            .current_dir(self.root.path().join("workspace"))
            .stdin(Stdio::null());
        command
    }

    fn cli(&self, scope: &Path, args: &[&str]) -> (i32, String, String) {
        self.cli_input(scope, args, &[], None, false)
    }

    fn cli_input(&self, scope: &Path, args: &[&str], environment: &[(&str, &str)], input: Option<&serde_json::Value>, fail_output: bool) -> (i32, String, String) {
        let id = self.sequence.fetch_add(1, Ordering::Relaxed);
        let stdout = self.root.path().join(format!("cli-{id}.out"));
        let stderr = self.root.path().join(format!("cli-{id}.err"));
        let mut command = self.command(scope);
        command.args(args).envs(environment.iter().copied()).stdout(fs::File::create(&stdout).unwrap()).stderr(fs::File::create(&stderr).unwrap());
        if input.is_some() {
            command.stdin(Stdio::piped());
        }
        if fail_output {
            command.stdout(fs::OpenOptions::new().write(true).open("/dev/full").unwrap());
        }
        let mut child = command.spawn().unwrap();
        if let Some(input) = input {
            use std::io::Write;
            let mut stdin = child.stdin.take().unwrap();
            stdin.write_all(&serde_json::to_vec(input).unwrap()).unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        let code = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status.code().unwrap_or(-1);
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("CLI deadline: {args:?}; stderr={}", fs::read_to_string(stderr).unwrap());
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        (code, fs::read_to_string(stdout).unwrap(), fs::read_to_string(stderr).unwrap())
    }

    fn spawn_daemon(&self, scope: &DaemonScope) -> OwnedDaemon {
        let id = self.sequence.fetch_add(1, Ordering::Relaxed);
        OwnedDaemon(self.command(scope.root()).args(["daemon", "serve", "--scope"]).arg(scope.root()).stdout(Stdio::null()).stderr(fs::File::create(self.root.path().join(format!("daemon-{id}.err"))).unwrap()).spawn().unwrap())
    }

    fn ready(&self, scope: &DaemonScope) -> DaemonInfo {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(info) = self.read_status(scope) {
                return info;
            }
            assert!(Instant::now() < deadline, "daemon never became ready; evidence at {}", self.root.path().display());
            std::thread::sleep(Duration::from_millis(15));
        }
    }

    fn read_status(&self, scope: &DaemonScope) -> Result<DaemonInfo, DaemonError> {
        let info = super::probe(scope)?;
        if info.build_id != self.build_id || info.build_version != BUILD_VERSION || info.protocol_version != PROTOCOL_VERSION {
            return Err(DaemonError::Protocol("fixture status identity mismatch".into()));
        }
        Ok(info)
    }

    fn call(&self, scope: &DaemonScope, caller: CallerContext, request: Request) -> Result<crate::service::ServiceResult<Response>, DaemonError> {
        super::call(scope, caller, request)
    }

    fn caller(&self, info: &DaemonInfo, request_id: &str) -> CallerContext {
        CallerContext {
            request_id: RequestId(request_id.into()),
            invocation: InvocationContext { cwd: self.root.path().join("workspace"), ..Default::default() },
            deadline_unix_ms: None,
            expected_database_generation: Some(info.database_generation.clone()),
        }
    }

    fn summary(&self, scope: &DaemonScope, info: &DaemonInfo) -> crate::service::maintenance::DatabaseSummary {
        match self.call(scope, self.caller(info, "summary"), Request::Maintenance(crate::service::maintenance::Request::Summary)).unwrap().unwrap() {
            Response::Maintenance(crate::service::maintenance::Response::Summary(summary)) => summary,
            other => panic!("wrong summary response: {other:?}"),
        }
    }

    fn register_hook(&self, scope: &DaemonScope, info: &DaemonInfo, tool: &str, binding: &str) -> crate::agents::attachments::ParticipantRef {
        use crate::agents::service::{Registration, Request as Participants, Response as ParticipantResponse};
        let mut caller = self.caller(info, &format!("register-{tool}"));
        caller.invocation.process_binding_id = Some(binding.into());
        let registration = Registration { name: None, tool: tool.into(), session_id: None, process_binding_id: Some(binding.into()), transcript_path: None, wait_timeout: 30, subagent_timeout: None, hints: None, auto_subscribe: String::new() };
        match self.call(scope, caller, Request::Participants(Participants::Register(registration))).unwrap().unwrap() {
            Response::Participants(ParticipantResponse::Registered(actor)) => actor.participant,
            other => panic!("native hook registration required: {other:?}"),
        }
    }

    fn hook_record(&self, scope: &DaemonScope, info: &DaemonInfo, name: &str) -> crate::db::InstanceRow {
        use crate::agents::service::{Request as Participants, Response as ParticipantResponse};
        match self.call(scope, self.caller(info, "hook-records"), Request::Participants(Participants::List)).unwrap().unwrap() {
            Response::Participants(ParticipantResponse::Records(records)) => records.into_iter().find(|row| row.name == name).expect("registered hook row"),
            other => panic!("native hook records required: {other:?}"),
        }
    }

    fn hook_message(&self, scope: &DaemonScope, info: &DaemonInfo, id: &str, name: &str, text: &str) -> i64 {
        use crate::messaging::service::{Request as Messages, Response as MessageResponse, Submission};
        let request = Messages::Submit(Submission { recipients: vec![name.into()], text: text.into(), envelope: Default::default(), external_sender: Some("fixture-human".into()), broadcast_confirmed: false, inline_bundle: None });
        match self.call(scope, self.caller(info, id), Request::Messages(request)).unwrap().unwrap() {
            Response::Messages(MessageResponse::Accepted(accepted)) => accepted.event.0,
            other => panic!("native fixture message acceptance required: {other:?}"),
        }
    }

    fn assert_daemon_descriptors(&self, scope: &DaemonScope, info: &DaemonInfo) {
        let database = self.summary(scope, info).database_path;
        let (handles, denied) = managed_descriptors(&database);
        println!("managed handles={handles:?}; inaccessible same-user PIDs={denied:?}");
        assert!(handles.iter().any(|(pid, _)| *pid == info.pid));
        assert!(handles.iter().all(|(pid, _)| *pid == info.pid));
    }

    fn daemons(&self) -> Vec<u32> {
        fs::read_dir("/proc")
            .unwrap()
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
                if fs::canonicalize(entry.path().join("exe")).ok()? != self.binary {
                    return None;
                }
                let bytes = fs::read(entry.path().join("cmdline")).ok()?;
                let args: Vec<_> = bytes.split(|byte| *byte == 0).collect();
                if args.len() < 5 || args[1..4] != [b"daemon".as_slice(), b"serve".as_slice(), b"--scope".as_slice()] {
                    return None;
                }
                let scope = PathBuf::from(std::ffi::OsStr::from_bytes(args[4]));
                scope.starts_with(self.root.path()).then_some(pid)
            })
            .collect()
    }

    /// Only the controlled launch's exact scope/environment and executable or
    /// script qualify. Recheck incarnation immediately before each signal.
    fn launched_processes(&self) -> Vec<(u32, String)> {
        let marker = format!("AGENTX_TEST_ROOT={}", self.root.path().display());
        let scope = format!("AGENTX_DIR={}", self.root.path().join("scope").display());
        let providers = ["bin/qodercli", "bin/codex"].map(|path| self.root.path().join(path));
        fs::read_dir("/proc")
            .unwrap()
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let pid = entry.file_name().to_str()?.parse::<u32>().ok()?;
                if crate::sys::process::has_exited(pid) {
                    return None;
                }
                let environment = fs::read(entry.path().join("environ")).ok()?;
                let variables: Vec<_> = environment.split(|byte| *byte == 0).collect();
                if !variables.contains(&marker.as_bytes()) || !variables.contains(&scope.as_bytes()) {
                    return None;
                }
                let executable = fs::canonicalize(entry.path().join("exe")).ok()?;
                let command = fs::read(entry.path().join("cmdline")).ok()?;
                let arguments: Vec<_> = command.split(|byte| *byte == 0).collect();
                let owned_script = arguments.iter().any(|argument| providers.iter().any(|provider| Path::new(std::ffi::OsStr::from_bytes(argument)) == provider));
                let owned_runner =
                    matches!(executable.file_name().and_then(|name| name.to_str()), Some("bash" | "dash" | "sh")) && arguments.iter().any(|argument| Path::new(std::ffi::OsStr::from_bytes(argument)).starts_with(self.root.path().join("scope/.tmp")));
                if executable != self.binary && !owned_script && !owned_runner {
                    return None;
                }
                Some((pid, crate::sys::process::identity(pid)?))
            })
            .collect()
    }

    fn stop_launched_processes(&self) {
        for signal in [libc::SIGTERM, libc::SIGKILL] {
            for (pid, incarnation) in self.launched_processes() {
                if crate::sys::process::identity(pid).as_deref() == Some(&incarnation) {
                    // SAFETY: exact disposable scope, executable/script and
                    // positive original incarnation were checked above.
                    unsafe {
                        libc::kill(pid as i32, signal);
                    }
                }
            }
            let deadline = Instant::now() + Duration::from_secs(2);
            while !self.launched_processes().is_empty() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        assert!(self.launched_processes().is_empty(), "controlled launch cleanup incomplete");
    }
}

/// The product's native cohort and full-byte audit are separate evidence.
/// Never substitute this debug harness's stamp for the selected product.
fn artifact_cohort(image: &[u8]) -> String {
    let pattern = regex::bytes::Regex::new(r"native-build-v1:[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}").unwrap();
    let stamps: std::collections::BTreeSet<_> = pattern.find_iter(image).map(|found| std::str::from_utf8(found.as_bytes()).unwrap().to_owned()).collect();
    assert_eq!(stamps.len(), 1, "fixed product must have exactly one distinct valid native cohort");
    stamps.into_iter().next().unwrap()
}

use std::os::unix::ffi::OsStrExt;

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop_launched_processes();
        for signal in [libc::SIGTERM, libc::SIGKILL] {
            for pid in self.daemons() {
                // SAFETY: fresh executable/scope checks restrict the target to
                // this fixture; these are never installed/live-scope daemons.
                unsafe {
                    libc::kill(pid as i32, signal);
                }
            }
            let deadline = Instant::now() + Duration::from_secs(2);
            while !self.daemons().is_empty() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        assert!(self.daemons().is_empty(), "fixture daemon cleanup incomplete");
    }
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux launch/PTY execution"]
fn process_qoder_real_launch_child_message_and_idle_wake() {
    controlled_pty_message_delivery("qoder", false, false);
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux launch/PTY execution"]
fn process_codex_message_delivery_idle_queue_and_reconnect() {
    controlled_pty_message_delivery("codex", false, false);
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux launch/PTY execution"]
fn process_codex_recover_continues_the_original_failed_session_once() {
    controlled_pty_message_delivery("codex", true, false);
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux launch/PTY execution"]
fn process_codex_missing_stop_reconciles_completed_turn_and_delivers_pending_mail() {
    controlled_pty_message_delivery("codex", false, true);
}

fn controlled_pty_message_delivery(tool: &str, recovery: bool, missed_stop: bool) {
    use crate::agents::service::{Request as Participants, Response as ParticipantResponse};
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let scope = fixture.scope("scope");
    fs::write(scope.root().join("config.toml"), "[launch]\nauto_trust_workspace = false\n[preferences]\nauto_approve = false\n").unwrap();
    let bin = fixture.root.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let provider = bin.join(if tool == "codex" { "codex" } else { "qodercli" });
    let provider_source = fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/daemon_qoder.py")).unwrap();
    println!("controlled Provider fixture sha256={}", super::hex_digest(&Sha256::digest(&provider_source)));
    fs::write(&provider, provider_source).unwrap();
    fs::set_permissions(&provider, fs::Permissions::from_mode(0o700)).unwrap();
    std::os::unix::fs::symlink(&fixture.binary, bin.join("agentx")).unwrap();
    // Supply a parent through the existing typed fixture registration; a bare
    // `start` has no Provider/session identity in this isolated operator env.
    let _daemon = fixture.spawn_daemon(&scope);
    let parent_info = fixture.ready(&scope);
    let parent_ref = fixture.register_hook(&scope, &parent_info, "adhoc", "controlled-parent");
    let parent = parent_ref.canonical_name.as_str();
    let root = fixture.root.path().to_str().unwrap();
    let project = fixture.root.path().join("workspace");
    let provider_home = fixture.root.path().join(format!("home/{tool}"));
    let path = format!("{}:/usr/bin:/bin", bin.display());
    let environment = [
        ("PATH", path.as_str()),
        ("AGENTX_TEST_ROOT", root),
        ("AGENTX_TEST_PARENT", parent),
        ("QODER_CONFIG_DIR", provider_home.to_str().unwrap()),
        ("CODEX_HOME", provider_home.to_str().unwrap()),
        ("AGENTX_TEST_RECOVERY", if recovery { "1" } else { "0" }),
        ("AGENTX_TEST_MISSED_STOP", if missed_stop { "1" } else { "0" }),
    ];
    let mut arguments = vec!["launch", "--tool", tool, "--headless", "--json", "--go", "--dir", project.to_str().unwrap(), "--iam", parent];
    if recovery {
        arguments.extend(["--label", "project=recover"]);
    }
    let (code, stdout, stderr) = fixture.cli_input(scope.root(), &arguments, &environment, None, false);
    assert_eq!(code, 0, "launch stdout={stdout}; stderr={stderr}");
    let launched: serde_json::Value = serde_json::from_str(stdout.trim()).expect("launch product JSON");
    assert_eq!(launched["launched"], 1, "{launched}");
    assert_eq!(launched["failed"], 0, "{launched}");
    let name = launched["names"][0].as_str().expect("launched participant");
    let info = fixture.ready(&scope);
    let observations = || -> Vec<serde_json::Value> { fs::read_to_string(fixture.root.path().join("qoder-observations.jsonl")).unwrap_or_default().lines().filter_map(|line| serde_json::from_str(line).ok()).collect() };
    let await_observation = |kind: &str| {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let rows = observations();
            if let Some(row) = rows.iter().find(|row| row["kind"] == kind) {
                break row.clone();
            }
            assert!(Instant::now() < deadline, "controlled child {kind} deadline; observations={rows:?}; launch={launched}");
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    await_observation("ready");
    let batch = launched["batch_id"].as_str().unwrap();
    // Query by batch without selecting the parent: an ordinary selected
    // participant can legitimately flush its pending child mail after JSON.
    let (code, ready, stderr) = fixture.cli(scope.root(), &["events", "launch", batch, "--timeout", "5"]);
    assert_eq!(code, 0, "product launch readiness={ready}; stderr={stderr}");
    assert_eq!(serde_json::from_str::<serde_json::Value>(ready.trim()).unwrap()["status"], "ready");
    let started = await_observation("started");
    let child_pid = u32::try_from(started["pid"].as_u64().unwrap()).unwrap();
    let inherited = &started["environment"];
    assert_eq!(inherited["AGENTX_LAUNCHED"], "1");
    assert_eq!(inherited["AGENTX_TOOL"], tool);
    assert_eq!(inherited["AGENTX_LAUNCH_GENERATION"], info.database_generation);
    assert!(inherited["AGENTX_PROCESS_ID"].as_str().is_some_and(|id| !id.is_empty()));
    let original: crate::agents::attachments::ParticipantRef = serde_json::from_str(inherited["AGENTX_LAUNCH_PARTICIPANT"].as_str().unwrap()).unwrap();
    assert_eq!(original.canonical_name, name);
    let record = || match fixture.call(&scope, fixture.caller(&info, "launched-record"), Request::Participants(Participants::Record { name: name.into() })).unwrap().unwrap() {
        Response::Participants(ParticipantResponse::Record(Some(record))) => record,
        other => panic!("wrong launched record: {other:?}"),
    };
    let before = record();
    assert_eq!(before.pid, Some(i64::from(child_pid)));
    assert_ne!(child_pid, info.pid);
    assert_eq!(before.directory, project.to_str().unwrap());
    assert_eq!(before.session_id.as_deref(), Some(format!("controlled-{tool}-session").as_str()));
    assert_eq!(before.status, "listening");
    let launch_context: serde_json::Value = serde_json::from_str(before.launch_context.as_deref().unwrap()).unwrap();
    let identity = crate::sys::process::identity(child_pid).expect("positive launched child incarnation");
    assert_eq!(launch_context["pid_identity"], identity);
    let control = fixture.root.path().join("qoder-control.json");
    let advance = |event: &str| {
        let temporary = control.with_extension("tmp");
        fs::write(&temporary, serde_json::json!({"event": event}).to_string()).unwrap();
        fs::rename(temporary, &control).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !observations().iter().any(|row| row["kind"] == "controlled-state" && row["command"]["event"] == event) {
            assert!(Instant::now() < deadline, "{event} was not handled: {:?}", observations());
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    if recovery || missed_stop {
        if missed_stop {
            advance("StartWithoutStop");
            assert_eq!(record().status_context, "model:request");
            let body = "pending mail after a missed Stop";
            let (code, output, stderr) = fixture.cli(scope.root(), &["send", "--json", "--iam", parent, &format!("@{name}"), "--", body]);
            assert_eq!(code, 0, "{output}; {stderr}");
            let sent: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
            assert_eq!(sent["delivery"]["recipients"][0]["mailbox"], "pending");
            advance("CompleteWithoutStop");
            assert_eq!(record().status, "active", "a draft cannot be overwritten");
            assert!(!observations().iter().any(|row| row["kind"] == "input"));
            advance("ClearDraft");
            let delivered = await_observation("delivered");
            assert!(delivered.to_string().contains(body), "{delivered}");
            let event = sent["event_id"].as_i64().unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            while record().last_event_id < event {
                assert!(Instant::now() < deadline, "receipt did not advance");
                std::thread::sleep(Duration::from_millis(20));
            }
            let after = record();
            assert_eq!((after.created_at.to_bits(), after.pid, after.session_id), (before.created_at.to_bits(), before.pid, before.session_id));
            assert_eq!(observations().iter().filter(|row| row["kind"] == "submitted").count(), 1);
            return;
        }
        let target = format!("@{name}");
        advance("Fail");
        assert_eq!(record().status, "error");
        let (code, output, stderr) = fixture.cli(scope.root(), &["recover", &target, "--json"]);
        assert_eq!(code, 1, "{output} {stderr}");
        assert_eq!(serde_json::from_str::<serde_json::Value>(&output).unwrap()[0]["status"], "skipped");
        advance("StopWithDraft");
        let (code, output, stderr) = fixture.cli(scope.root(), &["recover", "--all", "--label", "project=other", "--json"]);
        assert_eq!((code, output.trim()), (0, "[]"), "{stderr}");
        let (code, output, stderr) = fixture.cli(scope.root(), &["recover", "--all", "--label", "project=recover", "--label", "!blocked", "--json"]);
        assert_eq!(code, 0, "{output} {stderr}");
        let first = serde_json::from_str::<serde_json::Value>(&output).unwrap();
        assert_eq!(first[0]["status"], "queued");
        let (code, output, stderr) = fixture.cli(scope.root(), &["recover", &target, "--json"]);
        assert_eq!(code, 0, "{output} {stderr}");
        let duplicate = serde_json::from_str::<serde_json::Value>(&output).unwrap();
        assert_eq!(duplicate[0]["status"], "already_requested");
        assert_eq!(duplicate[0]["request_id"], first[0]["request_id"]);
        assert!(!observations().iter().any(|row| row["kind"] == "input"), "typed into an occupied prompt");
        advance("ClearDraft");
        let continued = await_observation("recovery-started");
        assert_eq!(continued["session"].as_str(), before.session_id.as_deref());
        let waiting = record();
        assert_eq!(waiting.status, "error", "starting another model request is not recovery evidence");
        assert_eq!((waiting.created_at.to_bits(), waiting.pid, waiting.last_event_id), (before.created_at.to_bits(), before.pid, before.last_event_id));
        advance("Success");
        let recovered = record();
        assert_eq!(recovered.status, "listening");
        assert!(recovered.model_request.unwrap().error.is_none());
        assert_eq!(observations().iter().filter(|row| row["kind"] == "submitted").count(), 1);
        return;
    }
    let (code, events, stderr) = fixture.cli(scope.root(), &["events", "--last", "30", "--full"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(events.lines().filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok()).any(|event| event["data"]["text"] == "controlled child to parent ready" && event["instance"] == name), "actual child message missing: {events}");
    let body = "controlled parent to idle child exact receipt";
    if tool == "codex" {
        advance("ShowPicker");
    }
    let (code, sent, stderr) = fixture.cli(scope.root(), &["send", "--json", "--iam", parent, &format!("@{name}"), "--", body]);
    assert_eq!(code, 0, "send={sent}; stderr={stderr}");
    let picker_closed = if tool == "codex" {
        let deadline = Instant::now() + Duration::from_secs(5);
        while record().status_context != "tui:input-overlay" {
            assert!(Instant::now() < deadline, "active picker was not classified: {:?}", observations());
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!observations().iter().any(|row| row["kind"] == "input"), "wrote at a menu");
        advance("ClosePicker");
        Some(Instant::now())
    } else {
        None
    };
    let delivered = await_observation("delivered");
    if let Some(picker_closed) = picker_closed {
        assert!(picker_closed.elapsed() < Duration::from_secs(2), "picker close did not wake the pending path promptly");
        assert_eq!(observations().iter().filter(|row| row["kind"] == "submitted").count(), 1, "must submit exactly one wake");
    }
    assert!(delivered["outputs"].as_array().unwrap().iter().any(|value| value["hookSpecificOutput"]["additionalContext"].as_str().is_some_and(|text| text.contains(body))), "wrong hook output: {delivered}");
    let deadline = Instant::now() + Duration::from_secs(5);
    let after = loop {
        let row = record();
        if row.last_event_id > before.last_event_id && row.status == "listening" {
            break row;
        }
        assert!(Instant::now() < deadline, "flush/Stop did not advance delivered cursor: {row:?}");
        std::thread::sleep(Duration::from_millis(20));
    };
    let (code, events, stderr) = fixture.cli(scope.root(), &["events", "--last", "30", "--full"]);
    assert_eq!(code, 0, "{stderr}");
    let accepted: serde_json::Value = events.lines().filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok()).find(|event| event["data"]["text"] == body).expect("accepted parent message event");
    assert_eq!(after.last_event_id, accepted["id"].as_i64().unwrap());
    // Caller health observes the exact accepted event and never drains or
    // resubmits it. Native output flush is the confirmation boundary.
    let first_event = accepted["id"].as_i64().unwrap().to_string();
    let delivery_report = |event: &str| {
        let (code, output, stderr) = fixture.cli(scope.root(), &["send", "--delivery-status", event, "--json"]);
        assert_eq!(code, 0, "delivery report={output}; {stderr}");
        serde_json::from_str::<serde_json::Value>(output.trim()).unwrap()
    };
    let confirmed = delivery_report(&first_event);
    assert_eq!(confirmed["recipients"][0]["mailbox"], "confirmed");
    assert_eq!(confirmed["recipients"][0]["participant"]["incarnation_bits"], original.incarnation_bits);
    assert_eq!(confirmed["recipients"][0]["host"]["child"]["pid"], child_pid);
    assert_eq!(confirmed["recipients"][0]["host"]["child"]["incarnation"], identity);
    assert!(confirmed["recipients"][0]["host"]["last_poll"].is_number());
    let parent_before = fixture.hook_record(&scope, &info, parent);
    let (code, _, stderr) = fixture.cli(scope.root(), &["send", "--delivery-status", &first_event, "--json", "--iam", parent]);
    assert_eq!(code, 0, "{stderr}");
    let parent_after = fixture.hook_record(&scope, &info, parent);
    assert_eq!((parent_after.last_event_id, &parent_after.status, &parent_after.status_context), (parent_before.last_event_id, &parent_before.status, &parent_before.status_context), "report lookup must not drain mail or change caller activity");

    advance("UserPromptSubmit");
    let queued_body = "queued during active turn; recover across daemon reconnect";
    let (code, output, stderr) = fixture.cli(scope.root(), &["send", "--json", "--iam", parent, &format!("@{name}"), "--", queued_body]);
    assert_eq!(code, 0, "{output}; {stderr}");
    let queued: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    let queued_event = queued["event_id"].as_i64().unwrap().to_string();
    assert_eq!(queued["delivery"]["recipients"][0]["mailbox"], "pending");
    assert_eq!(observations().iter().filter(|row| row["kind"] == "submitted").count(), 1, "active turn must not receive a second wake");
    let (code, output, stderr) = fixture.cli(scope.root(), &["daemon", "restart", "--scope", scope.root().to_str().unwrap()]);
    assert_eq!(code, 0, "{output}; {stderr}");
    let restarted = fixture.ready(&scope);
    assert_ne!(restarted.daemon_instance, info.daemon_instance);
    assert_eq!(restarted.database_generation, info.database_generation);
    assert_eq!(record().pid, before.pid, "reconnect must retain the waiting child");
    assert_eq!(delivery_report(&queued_event)["recipients"][0]["mailbox"], "pending");
    fs::write(&control, r#"{"event":"Stop"}"#).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let rows = observations();
        if rows.iter().filter(|row| row["kind"] == "delivered").count() == 2 {
            assert!(rows.iter().filter(|row| row["kind"] == "delivered").any(|row| row.to_string().contains(queued_body)));
            break;
        }
        assert!(Instant::now() < deadline, "active-to-idle/reconnect delivery stalled: {rows:?}");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(delivery_report(&queued_event)["recipients"][0]["mailbox"], "confirmed");
    let activations = observations().iter().filter(|row| row["kind"] == "submitted" || row["kind"] == "continued").count();
    assert_eq!(activations, 2, "one activation per delivered batch");
    // Native explicit recovery of a stopped host must retain the Provider
    // session/backlog. No terminal injection or duplicate send is involved.
    fs::write(&control, r#"{"event":"UserPromptSubmit","sequence":2}"#).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while record().status != "active" {
        assert!(Instant::now() < deadline, "controlled second active turn not observed");
        std::thread::sleep(Duration::from_millis(20));
    }
    let recovery_body = "durable recovery sentinel accepted before native host retirement";
    let (code, output, stderr) = fixture.cli(scope.root(), &["send", "--json", "--iam", parent, &format!("@{name}"), "--", recovery_body]);
    assert_eq!(code, 0, "{output}; {stderr}");
    let recovery: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(recovery["delivery"]["recipients"][0]["mailbox"], "pending");
    let (code, output, stderr) = fixture.cli(scope.root(), &["kill", name, "--json"]);
    assert_eq!(code, 0, "native retirement={output}; {stderr}");
    fs::write(&control, r#"{"event":"Stop","sequence":3}"#).unwrap();
    let (code, output, stderr) = fixture.cli_input(scope.root(), &["resume", name, "--headless", "--go", "--json"], &environment, None, false);
    assert_eq!(code, 0, "native resume={output}; {stderr}");
    let resumed: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    let resumed_name = resumed["names"][0].as_str().expect("resumed participant");
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let rows = observations();
        let outputs: Vec<_> = rows.iter().filter(|row| row["kind"] == "delivered" || row["kind"] == "session-started").collect();
        if outputs.iter().any(|row| row.to_string().contains(recovery_body)) {
            break;
        }
        assert!(Instant::now() < deadline, "resumed durable backlog did not reach Provider: {rows:?}");
        std::thread::sleep(Duration::from_millis(20));
    }
    let resumed_record = match fixture.call(&scope, fixture.caller(&restarted, "recovered-record"), Request::Participants(Participants::Record { name: resumed_name.into() })).unwrap().unwrap() {
        Response::Participants(ParticipantResponse::Record(Some(record))) => record,
        other => panic!("wrong recovered record: {other:?}"),
    };
    assert_eq!(resumed_record.session_id, before.session_id);
    assert!(resumed_record.last_event_id >= recovery["event_id"].as_i64().unwrap());
    assert_ne!(resumed_record.pid, before.pid);
    let database = fixture.summary(&scope, &info).database_path;
    let (handles, denied) = managed_descriptors(&database);
    let stages: Vec<_> = observations().into_iter().filter(|row| matches!(row["kind"].as_str(), Some("submitted" | "continued" | "delivered" | "controlled-state"))).collect();
    println!(
        "MESSAGE_DELIVERY_EVIDENCE {}",
        serde_json::json!({"tool":tool,"artifact":fixture.binary,"sha256":fixture.artifact_sha256,"original":original,"provider_session":before.session_id,"child_pid":child_pid,"child_identity":identity,"resumed_name":resumed_name,"resumed_pid":resumed_record.pid,"daemon_before":info.pid,"daemon_after":restarted.pid,"database_generation":restarted.database_generation,"first_report":confirmed,"queued_event":queued_event,"recovery_event":recovery["event_id"],"stages":stages,"managed_handles":handles,"inaccessible_processes":denied})
    );
    assert!(handles.iter().any(|(pid, _)| *pid == restarted.pid));
    assert!(handles.iter().all(|(pid, _)| *pid == restarted.pid));
    assert!(!fixture.launched_processes().is_empty());

    if tool == "codex" {
        advance("HoldNextReceipt");
        let submissions_before = observations().iter().filter(|row| row["kind"] == "submitted").count();
        let target = format!("@{resumed_name}");
        let (code, output, stderr) = fixture.cli(scope.root(), &["send", "--json", "--iam", parent, &target, "--", "accepted without a hook receipt"]);
        assert_eq!(code, 0, "{output}; {stderr}");
        let event = serde_json::from_str::<serde_json::Value>(&output).unwrap()["event_id"].as_i64().unwrap().to_string();
        await_observation("submitted-without-receipt");
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let report = delivery_report(&event);
            assert_eq!(report["recipients"][0]["mailbox"], "pending", "{report}");
            assert_eq!(observations().iter().filter(|row| row["kind"] == "submitted").count(), submissions_before + 1, "the host may not retype an unconfirmed wake");
            if report["recipients"][0]["context"] == "tui:wake-unacknowledged" {
                break;
            }
            assert!(Instant::now() < deadline, "unconfirmed wake did not latch: {report}");
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

/// Snapshot every readable same-user process descriptor referring to the exact
/// managed DB/WAL/SHM names. Permission-denied same-user processes are retained
/// as evidence gaps rather than counted as closed descriptors.
fn managed_descriptors(database: &Path) -> (Vec<(u32, PathBuf)>, Vec<u32>) {
    use std::os::unix::fs::MetadataExt;
    let mut handles = Vec::new();
    let mut denied = Vec::new();
    let names = [database.to_path_buf(), PathBuf::from(format!("{}-wal", database.display())), PathBuf::from(format!("{}-shm", database.display()))];
    for proc in fs::read_dir("/proc").unwrap().filter_map(Result::ok) {
        let Ok(pid) = proc.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let Ok(metadata) = proc.metadata() else {
            continue;
        };
        // SAFETY: geteuid has no preconditions.
        if metadata.uid() != unsafe { libc::geteuid() } {
            continue;
        }
        let descriptors = match fs::read_dir(proc.path().join("fd")) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                denied.push(pid);
                continue;
            }
            Err(_) => continue,
        };
        for entry in descriptors.filter_map(Result::ok) {
            if let Ok(target) = fs::read_link(entry.path())
                && names.contains(&target)
            {
                handles.push((pid, target));
            }
        }
    }
    (handles, denied)
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux process execution"]
fn process_concurrent_start_alias_scope_and_database_owner() {
    let fixture = Fixture::new();
    let scope = fixture.scope("scope");
    let alias = fixture.root.path().join("alias");
    std::os::unix::fs::symlink(scope.root(), &alias).unwrap();
    std::thread::scope(|threads| {
        let tasks: Vec<_> = (0..8)
            .map(|index| {
                let fixture = &fixture;
                let path = if index % 2 == 0 { scope.root() } else { &alias };
                threads.spawn(move || {
                    let (code, stdout, stderr) = fixture.cli(path, &["list", "--json"]);
                    assert_eq!(code, 0, "{stderr}");
                    assert_eq!(serde_json::from_str::<serde_json::Value>(&stdout).unwrap(), serde_json::json!([]));
                })
            })
            .collect();
        for task in tasks {
            task.join().unwrap();
        }
    });
    let info = fixture.ready(&scope);
    let alias_info = fixture.ready(&DaemonScope::prepare(&alias).unwrap());
    assert_eq!(info.daemon_instance, alias_info.daemon_instance);
    assert_eq!(info.pid, alias_info.pid);
    let second = fixture.scope("second");
    let (code, _, stderr) = fixture.cli(second.root(), &["list", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    let second_info = fixture.ready(&second);
    assert_ne!(info.pid, second_info.pid);
    let summary = fixture.summary(&scope, &info);
    let (handles, denied) = managed_descriptors(&summary.database_path);
    println!("DD-01/DD-11 managed handles={handles:?}; inaccessible same-user PIDs={denied:?}");
    assert!(handles.iter().any(|(pid, _)| *pid == info.pid), "daemon had no managed DB handle");
    assert!(handles.iter().all(|(pid, _)| *pid == info.pid), "non-daemon opened managed DB: {handles:?}");
    assert_eq!(fixture.daemons().len(), 2);
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux process execution"]
fn process_restart_retains_generation_reset_rejects_stale_client() {
    let fixture = Fixture::new();
    let scope = fixture.scope("scope");
    let mut first = fixture.spawn_daemon(&scope);
    let before = fixture.ready(&scope);
    let (code, _, stderr) = fixture.cli(scope.root(), &["start"]);
    assert_eq!(code, 0, "{stderr}");
    let output = fixture.root.path().join("following-events.out");
    let errors = fixture.root.path().join("following-events.err");
    let mut follower = OwnedDaemon(fixture.command(scope.root()).args(["events", "--follow", "--last", "1", "--full"]).stdout(fs::File::create(&output).unwrap()).stderr(fs::File::create(&errors).unwrap()).spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(8);
    while fs::metadata(&output).unwrap().len() == 0 {
        assert!(follower.try_wait().unwrap().is_none(), "event follower exited before readback: {}", fs::read_to_string(&errors).unwrap());
        assert!(Instant::now() < deadline, "event follower never emitted its initial page");
        std::thread::sleep(Duration::from_millis(10));
    }
    let database = fixture.summary(&scope, &before).database_path;
    let (handles, denied) = managed_descriptors(&database);
    println!("DD-09 event follower PID={} managed handles={handles:?}; inaccessible same-user PIDs={denied:?}", follower.id());
    assert!(handles.iter().all(|(pid, _)| *pid == before.pid));
    first.kill().unwrap();
    first.wait().unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    let status = loop {
        if let Some(status) = follower.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "event client did not report daemon disappearance");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(!status.success(), "event client hid daemon disappearance");
    assert!(fixture.daemons().is_empty(), "event client unexpectedly restarted daemon");
    assert!(managed_descriptors(&database).0.is_empty(), "client opened managed DB after daemon loss");
    let _second = fixture.spawn_daemon(&scope);
    let after = fixture.ready(&scope);
    assert_ne!(before.daemon_instance, after.daemon_instance);
    assert_eq!(before.database_generation, after.database_generation);
    let reset = fixture.call(&scope, fixture.caller(&after, "reset"), Request::Maintenance(crate::service::maintenance::Request::ResetConversation)).unwrap().unwrap();
    let Response::Maintenance(crate::service::maintenance::Response::Reset(reset)) = reset else {
        panic!("wrong reset response");
    };
    assert_ne!(after.database_generation, reset.database_generation);
    let result = fixture.call(&scope, fixture.caller(&after, "old-generation"), Request::Maintenance(crate::service::maintenance::Request::Summary));
    assert!(matches!(result, Err(DaemonError::StaleGeneration) | Ok(Err(crate::service::ServiceError::StaleGeneration))), "stale client was admitted: {result:?}");
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux process execution"]
fn process_direct_wire_rejects_an_unknown_operation_and_serves_shared_operations() {
    let fixture = Fixture::new();
    let scope = fixture.scope("scope");
    let _daemon = fixture.spawn_daemon(&scope);
    let info = fixture.ready(&scope);
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut stream = super::platform::connect(&scope, deadline).unwrap();
    super::frame::write(&mut stream, &ClientFrame::Call { canonical_scope: scope.root().into(), daemon_instance: None, caller: fixture.caller(&info, "future-operation"), request: serde_json::json!({"Events":"FutureEventOperation"}) }, deadline)
        .unwrap();
    assert!(matches!(super::frame::read::<ServerFrame>(&mut stream, deadline).unwrap(), ServerFrame::Failure(DaemonError::Unsupported(operation)) if operation == "Events.FutureEventOperation"));
    assert_eq!(fixture.summary(&scope, &info).event_count, 0);
}

#[test]
#[ignore = "requires fixed cohort product and a separately built test harness"]
fn process_native_cohort_copy_audit_and_running_path_replacement() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    let mut fixture = Fixture::new();
    let copy = fixture.root.path().join("copied-agentx");
    fs::copy(&fixture.binary, &copy).unwrap();
    fs::set_permissions(&copy, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(artifact_cohort(&fs::read(&copy).unwrap()), fixture.build_id);
    // A post-link change is an artifact-audit difference. Native compatibility
    // intentionally does not promise runtime byte-tamper detection.
    fs::OpenOptions::new().append(true).open(&copy).unwrap().write_all(b"\0disposable artifact audit difference\0").unwrap();
    let audited = fs::read(&copy).unwrap();
    assert_eq!(artifact_cohort(&audited), fixture.build_id);
    let changed_sha: String = Sha256::digest(&audited).iter().map(|byte| format!("{byte:02x}")).collect();
    assert_ne!(changed_sha, fixture.artifact_sha256);
    fixture.binary = copy.clone();
    let scope = fixture.scope("scope");
    let _daemon = fixture.spawn_daemon(&scope);
    let original = fixture.ready(&scope);
    let replacement = fixture.root.path().join("replacement-agentx");
    let harness = std::env::current_exe().unwrap();
    let replacement_cohort = artifact_cohort(&fs::read(&harness).unwrap());
    assert_ne!(replacement_cohort, fixture.build_id, "profile-separated harness must be a different actual cohort");
    fs::copy(harness, &replacement).unwrap();
    fs::rename(replacement, &copy).unwrap();
    assert_eq!(artifact_cohort(&fs::read(&copy).unwrap()), replacement_cohort);
    // The original running inode retains its own compiled native stamp.
    let after = fixture.ready(&scope);
    assert_eq!(after.pid, original.pid);
    assert_eq!(after.build_id, original.build_id);
    assert_eq!(fixture.summary(&scope, &after).event_count, 0);
    println!("native copied cohort={} postlink audit_sha256={changed_sha}; running replacement retained pid={} cohort={}", fixture.build_id, after.pid, after.build_id);
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux process execution"]
fn process_abandoned_reply_reconciles_after_restart_without_duplicate_message() {
    use crate::messaging::service::{Request as MessageRequest, Response as MessageResponse, Submission};
    let fixture = Fixture::new();
    let scope = fixture.scope("scope");
    let mut daemon = fixture.spawn_daemon(&scope);
    let info = fixture.ready(&scope);
    let mut names = Vec::new();
    for _ in 0..2 {
        let (code, stdout, stderr) = fixture.cli(scope.root(), &["start"]);
        assert_eq!(code, 0, "{stderr}");
        names.push(stdout.lines().find_map(|line| line.trim_start().strip_prefix("[agentx:").and_then(|tail| tail.split(']').next())).expect("start participant marker").to_owned());
    }
    let target = format!("@{}", names[1]);
    let cli_body = "actual CLI send observed by actual CLI events";
    let (code, _, stderr) = fixture.cli(scope.root(), &["--iam", &names[0], "send", "--json", &target, "--", cli_body]);
    assert_eq!(code, 0, "{stderr}");
    let (code, events, stderr) = fixture.cli(scope.root(), &["events", "--last", "20", "--full"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(events.lines().filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok()).any(|event| event["data"]["text"].as_str() == Some(cli_body)), "CLI event projection lost sent message: {events}");
    let before = fixture.summary(&scope, &info).event_count;
    let mut caller = fixture.caller(&info, "stable-lost-reply");
    caller.invocation.explicit_name = Some(names[0].clone());
    let submission = Submission { recipients: vec![names[1].clone()], text: "one durable frozen message".into(), envelope: Default::default(), external_sender: None, broadcast_confirmed: false, inline_bundle: None };
    let mut stream = super::platform::connect(&scope, Instant::now() + Duration::from_secs(3)).unwrap();
    super::frame::write(
        &mut stream,
        &ClientFrame::Call { canonical_scope: scope.root().into(), daemon_instance: Some(info.daemon_instance.clone()), caller: caller.clone(), request: Request::Messages(MessageRequest::Submit(submission.clone())) },
        Instant::now() + Duration::from_secs(3),
    )
    .unwrap();
    // Deliberately abandon the only reply. A successful socket write says
    // nothing about commit; the durable acceptance lookup below is authoritative.
    drop(stream);
    let deadline = Instant::now() + Duration::from_secs(5);
    let accepted = loop {
        let response = fixture.call(&scope, caller.clone(), Request::Messages(MessageRequest::Acceptance { request_id: caller.request_id.clone() })).unwrap().unwrap();
        if let Response::Messages(MessageResponse::Acceptance(Some(accepted))) = response {
            break accepted;
        }
        assert!(Instant::now() < deadline, "abandoned request never reached durable acceptance");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(fixture.summary(&scope, &info).event_count, before + 1);
    daemon.kill().unwrap();
    daemon.wait().unwrap();
    let _successor = fixture.spawn_daemon(&scope);
    let restarted = fixture.ready(&scope);
    assert_eq!(info.database_generation, restarted.database_generation);
    let replay = fixture.call(&scope, caller.clone(), Request::Messages(MessageRequest::Submit(submission.clone()))).unwrap().unwrap();
    let Response::Messages(MessageResponse::Accepted(replay)) = replay else {
        panic!("expected durable replay");
    };
    assert_eq!(accepted.event, replay.event);
    assert_eq!(accepted.recipients, replay.recipients);
    assert!(replay.replayed);
    let mut changed = submission;
    changed.text = "conflicting repeated request".into();
    let conflict = fixture.call(&scope, caller, Request::Messages(MessageRequest::Submit(changed))).unwrap();
    assert!(matches!(conflict, Err(crate::service::ServiceError::Conflict(_))), "changed retry input was accepted: {conflict:?}");
    assert_eq!(fixture.summary(&scope, &restarted).event_count, before + 1);
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux process execution"]
fn process_originating_environment_selects_actor_instead_of_daemon_environment() {
    use crate::agents::service::{Request as ParticipantRequest, Response as ParticipantResponse};
    let fixture = Fixture::new();
    let scope = fixture.scope("scope");
    let _daemon = fixture.spawn_daemon(&scope);
    let info = fixture.ready(&scope);
    let mut participants = Vec::new();
    for index in 0..2 {
        let binding = format!("fixture-caller-{index}");
        let mut caller = fixture.caller(&info, &format!("register-{index}"));
        caller.invocation.process_binding_id = Some(binding.clone());
        caller.invocation.cwd = fixture.root.path().join(if index == 0 { "home" } else { "workspace" });
        caller.invocation.environment.insert("AGENTX_PROCESS_ID".into(), binding.clone());
        let registration =
            crate::agents::service::Registration { name: None, tool: "codex".into(), session_id: None, process_binding_id: Some(binding), transcript_path: None, wait_timeout: 30, subagent_timeout: None, hints: None, auto_subscribe: String::new() };
        let response = fixture.call(&scope, caller.clone(), Request::Participants(ParticipantRequest::Register(registration))).unwrap().unwrap();
        let Response::Participants(ParticipantResponse::Registered(actor)) = response else {
            panic!("registration response");
        };
        let resolved = fixture.call(&scope, caller, Request::Participants(ParticipantRequest::Resolve)).unwrap().unwrap();
        let Response::Participants(ParticipantResponse::Actor(resolved)) = resolved else {
            panic!("resolved actor response");
        };
        assert_eq!(actor.participant, resolved.participant);
        participants.push(actor.participant);
    }
    assert_ne!(participants[0], participants[1]);
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux process execution"]
fn process_qoder_hook_ipc_confirms_only_flushed_bootstrap_and_mailbox() {
    use crate::agents::service::{Request as Participants, Response as ParticipantResponse};
    use crate::messaging::service::{Request as Messages, Response as MessageResponse, Submission};
    let fixture = Fixture::new();
    let scope = fixture.scope("scope");
    let environment = [("AGENTX_PROCESS_ID", "fixture-qoder-process"), ("AGENTX_TOOL", "qoder"), ("AGENTX_LAUNCHED", "1")];
    let payload = serde_json::json!({"session_id":"fixture-qoder-session","source":"startup","prompt":"fixture prompt"});
    let (code, _, stderr) = fixture.cli_input(scope.root(), &["qoder-sessionstart"], &environment, Some(&payload), true);
    assert_eq!(code, 0, "{stderr}");
    let info = fixture.ready(&scope);
    let records = || {
        let Response::Participants(ParticipantResponse::Records(records)) = fixture.call(&scope, fixture.caller(&info, "hook-records"), Request::Participants(Participants::List)).unwrap().unwrap() else {
            panic!("participant records");
        };
        records
    };
    assert_eq!(records().len(), 1, "failed output must still preserve accepted identity");
    assert_eq!(records()[0].name_announced, 0, "failed bootstrap output was confirmed");
    let (code, stdout, stderr) = fixture.cli_input(scope.root(), &["qoder-sessionstart"], &environment, Some(&payload), false);
    assert_eq!(code, 0, "{stderr}");
    let output: serde_json::Value = serde_json::from_str(&stdout).expect("actual hook must emit bootstrap JSON");
    assert!(output["hookSpecificOutput"]["additionalContext"].as_str().is_some_and(|text| !text.is_empty()));
    let initial = records();
    assert_eq!(initial.len(), 1);
    let name = initial[0].name.clone();
    assert_eq!(initial[0].tool, "qoder");
    assert_eq!(initial[0].name_announced, 1, "successful bootstrap flush was not confirmed");
    assert_ne!(initial[0].pid, Some(i64::from(info.pid)), "daemon PID must not impersonate Provider");
    let mut cursor = initial[0].last_event_id;
    for (index, fail_output) in [true, false].into_iter().enumerate() {
        let body = format!("exact Qoder IPC mailbox fixture {index}");
        let Response::Messages(MessageResponse::Accepted(accepted)) = fixture
            .call(
                &scope,
                fixture.caller(&info, &format!("qoder-mail-{index}")),
                Request::Messages(Messages::Submit(Submission { recipients: vec![name.clone()], text: body.clone(), envelope: Default::default(), external_sender: Some("fixture-human".into()), broadcast_confirmed: false, inline_bundle: None })),
            )
            .unwrap()
            .unwrap()
        else {
            panic!("accepted fixture mail");
        };
        let (code, stdout, stderr) = fixture.cli_input(scope.root(), &["qoder-userpromptsubmit"], &environment, Some(&payload), fail_output);
        assert_eq!(code, 0, "{stderr}");
        if fail_output {
            assert_eq!(records()[0].last_event_id, cursor, "failed output advanced mailbox cursor");
            let (code, retried, stderr) = fixture.cli_input(scope.root(), &["qoder-userpromptsubmit"], &environment, Some(&payload), false);
            assert_eq!(code, 0, "{stderr}");
            assert!(retried.contains(&body), "unflushed mail disappeared on retry");
        } else {
            assert!(stdout.contains(&body));
        }
        cursor = accepted.event.0;
        assert_eq!(records()[0].last_event_id, cursor);
    }
    let database = fixture.summary(&scope, &info).database_path;
    let (handles, denied) = managed_descriptors(&database);
    println!("DD-10 Qoder native hook handles={handles:?}; inaccessible same-user PIDs={denied:?}");
    assert!(handles.iter().any(|(pid, _)| *pid == info.pid));
    assert!(handles.iter().all(|(pid, _)| *pid == info.pid));
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux process execution"]
fn process_events_archive_continuation_and_warning_modes() {
    let fixture = Fixture::new();
    let scope = fixture.scope("scope");
    let archived = scope.root().join("archive/session-product");
    fs::create_dir_all(&archived).unwrap();
    let count = crate::events::service::MAX_PAGE_EVENTS + 7;
    // Seed fixtures before any daemon starts. Runtime CLI/observers never get
    // a managed handle; archive/current IDs deliberately collide at every row.
    for directory in [scope.root(), archived.as_path()] {
        let db = crate::db::AgentxDb::open_at(&directory.join("agentx.db")).unwrap();
        db.with_write_scope(|| {
            for _ in 0..count {
                db.conn().execute("INSERT INTO events(timestamp,type,instance,data) VALUES('2026-11-07T00:00:00Z','message','history-fixture','{}')", [])?;
            }
            Ok(())
        })
        .unwrap();
    }
    let daemon = fixture.spawn_daemon(&scope);
    let info = fixture.ready(&scope);
    let total = (count * 2).to_string();
    let (code, stdout, stderr) = fixture.cli(scope.root(), &["events", "--all", "--last", &total, "--full"]);
    assert_eq!(code, 0, "{stderr}");
    let events: Vec<serde_json::Value> = stdout.lines().map(|line| serde_json::from_str(line).expect("actual CLI JSON event")).collect();
    assert_eq!(events.len(), count * 2, "actual CLI truncated merged archive history");
    for source in ["current", "session-product"] {
        assert_eq!(events.iter().filter(|event| event["source"] == source).count(), count);
    }
    let keys: Vec<_> = events.iter().map(|event| (event["ts"].as_str().unwrap(), event["source"].as_str().unwrap(), event["id"].as_i64().unwrap())).collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "source/id ties must not drop, duplicate or reorder events");
    let (code, _, warning) = fixture.cli(scope.root(), &["events", "--agent", "missing-history-agent"]);
    assert_eq!(code, 0, "{warning}");
    assert_eq!(warning.matches("No agent named 'missing-history-agent'").count(), 1);
    let (code, _, warning) = fixture.cli(scope.root(), &["events", "--all", "--agent", "missing-history-agent"]);
    assert_eq!(code, 0, "{warning}");
    assert!(!warning.contains("No agent named"), "archive mode must suppress ordinary unknown-agent warnings");
    fixture.assert_daemon_descriptors(&scope, &info);
    drop(daemon);
    let db = crate::db::AgentxDb::open_at(&scope.root().join("agentx.db")).unwrap();
    assert!(db.kv_prefix("event_history:").unwrap().is_empty(), "actual CLI did not end its observed history snapshot");
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux process execution"]
fn process_codex_hook_session_isolation_patch_paths_and_stdout_receipts() {
    let fixture = Fixture::new();
    let scope = fixture.scope("scope");
    let _daemon = fixture.spawn_daemon(&scope);
    let info = fixture.ready(&scope);
    let binding = "fixture-codex-hooks";
    let participant = fixture.register_hook(&scope, &info, "codex", binding);
    let name = participant.canonical_name.as_str();
    let environment = [("AGENTX_PROCESS_ID", binding), ("AGENTX_TOOL", "codex"), ("AGENTX_LAUNCHED", "1")];
    let sidecar = scope.root().join(".tmp/terminal_ids").join(binding);
    fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
    fs::write(&sidecar, "controlled-terminal-id").unwrap();
    let initial = fixture.hook_record(&scope, &info, name);
    let memory = serde_json::json!({"session_id":"internal-memory","source":"startup","cwd":fixture.root.path().join("home/codex/memories_v2")});
    let (code, output, stderr) = fixture.cli_input(scope.root(), &["codex-sessionstart"], &environment, Some(&memory), false);
    assert_eq!(code, 0, "{stderr}");
    assert!(output.trim().is_empty());
    assert_eq!(serde_json::to_value(fixture.hook_record(&scope, &info, name)).unwrap(), serde_json::to_value(&initial).unwrap());
    assert!(sidecar.exists(), "internal memory hook consumed root terminal handoff");
    let transcript = fixture.root.path().join("home/codex/main-session.jsonl");
    let main = serde_json::json!({"session_id":"codex-main","source":"startup","transcript_path":transcript,"cwd":fixture.root.path().join("workspace")});
    let (code, _, stderr) = fixture.cli_input(scope.root(), &["codex-sessionstart"], &environment, Some(&main), false);
    assert_eq!(code, 0, "{stderr}");
    let bound = fixture.hook_record(&scope, &info, name);
    assert_eq!(bound.session_id.as_deref(), Some("codex-main"));
    assert_eq!(bound.transcript_path, transcript.to_str().unwrap());
    assert!(!sidecar.exists(), "main session did not consume its terminal handoff");
    let before_internal = serde_json::to_value(&bound).unwrap();
    for event in ["codex-sessionstart", "codex-userpromptsubmit", "codex-pretooluse", "codex-posttooluse"] {
        let (code, _, stderr) = fixture.cli_input(scope.root(), &[event], &environment, Some(&memory), false);
        assert_eq!(code, 0, "{stderr}");
        assert_eq!(serde_json::to_value(fixture.hook_record(&scope, &info, name)).unwrap(), before_internal);
    }
    let event = fixture.hook_message(&scope, &info, "codex-mail", name, "exact Codex flushed context");
    let mut prompt = main.clone();
    prompt["prompt"] = serde_json::json!("ordinary human prompt");
    let (_, output, _) = fixture.cli_input(scope.root(), &["codex-userpromptsubmit"], &environment, Some(&prompt), false);
    assert!(output.trim().is_empty(), "ordinary human prompt received mailbox output");
    assert_eq!(fixture.hook_record(&scope, &info, name).last_event_id, bound.last_event_id);
    prompt["prompt"] = serde_json::json!("<agentx>");
    fixture.cli_input(scope.root(), &["codex-userpromptsubmit"], &environment, Some(&prompt), true);
    assert_eq!(fixture.hook_record(&scope, &info, name).last_event_id, bound.last_event_id);
    let (code, output, stderr) = fixture.cli_input(scope.root(), &["codex-userpromptsubmit"], &environment, Some(&prompt), false);
    assert_eq!(code, 0, "{stderr}");
    let output: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(output["systemMessage"], output["hookSpecificOutput"]["additionalContext"]);
    assert!(output["systemMessage"].as_str().unwrap().contains("exact Codex flushed context"));
    assert_eq!(fixture.hook_record(&scope, &info, name).last_event_id, event);
    let mut patch = main.clone();
    patch["tool_name"] = serde_json::json!("apply_patch");
    patch["tool_input"] = serde_json::json!({"command":"*** Begin Patch\n*** Add File: first.rs\n+one\n*** Update File: second.rs\n@@\n-old\n+new\n*** End Patch"});
    let (code, _, stderr) = fixture.cli_input(scope.root(), &["codex-pretooluse"], &environment, Some(&patch), false);
    assert_eq!(code, 0, "{stderr}");
    let query = crate::events::service::EventQuery { predicate: Some(format!("instance = '{name}' AND json_extract(data,'$.context') = 'tool:apply_patch'")), after: Some(event), limit: 100, oldest_first: true, ..Default::default() };
    let Response::Events(crate::events::service::Response::Page(page)) = fixture.call(&scope, fixture.caller(&info, "patch-events"), Request::Events(crate::events::service::Request::Query(query))).unwrap().unwrap() else {
        panic!("patch event page required");
    };
    let paths: Vec<_> = page.events.iter().filter_map(|value| value["data"]["detail"].as_str()).collect();
    assert_eq!(paths, vec!["first.rs", "second.rs"]);
    let (code, output, stderr) = fixture.cli_input(scope.root(), &["codex-stop"], &environment, Some(&main), false);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(serde_json::from_str::<serde_json::Value>(&output).unwrap(), serde_json::json!({}));
    assert_eq!(fixture.hook_record(&scope, &info, name).status, "listening");
    let (code, output, stderr) = fixture.cli_input(scope.root(), &["codex-interrupt"], &environment, Some(&main), false);
    assert_eq!(code, 0, "{stderr}");
    assert!(output.trim().is_empty());
    assert_eq!(fixture.hook_record(&scope, &info, name).status_context, "interrupted");
    fixture.assert_daemon_descriptors(&scope, &info);
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux process execution"]
fn process_copilot_hooks_and_extension_confirm_only_exact_actual_prompt_or_flush() {
    let fixture = Fixture::new();
    let scope = fixture.scope("scope");
    let _daemon = fixture.spawn_daemon(&scope);
    let info = fixture.ready(&scope);
    let binding = "fixture-copilot-hooks";
    let participant = fixture.register_hook(&scope, &info, "copilot", binding);
    let name = participant.canonical_name.as_str();
    let environment = [("AGENTX_PROCESS_ID", binding), ("AGENTX_TOOL", "copilot"), ("AGENTX_LAUNCHED", "1")];
    let payload = serde_json::json!({"session_id":"copilot-main","source":"startup","cwd":fixture.root.path().join("workspace")});
    fixture.cli_input(scope.root(), &["copilot-sessionstart"], &environment, Some(&payload), true);
    assert_eq!(fixture.hook_record(&scope, &info, name).name_announced, 0);
    let (code, output, stderr) = fixture.cli_input(scope.root(), &["copilot-sessionstart"], &environment, Some(&payload), false);
    assert_eq!(code, 0, "{stderr}");
    assert!(serde_json::from_str::<serde_json::Value>(&output).unwrap()["additionalContext"].as_str().is_some_and(|text| !text.is_empty()));
    let initial = fixture.hook_record(&scope, &info, name);
    assert_eq!(initial.name_announced, 1);
    let mut prompt = payload.clone();
    prompt["prompt"] = serde_json::json!("ordinary human prompt");
    fixture.cli_input(scope.root(), &["copilot-userpromptsubmit"], &environment, Some(&prompt), false);
    fixture.cli_input(scope.root(), &["copilot-sessionstart"], &environment, Some(&payload), false);
    assert_eq!(fixture.hook_record(&scope, &info, name).status, "active", "lazy SessionStart idled an active turn");
    let event = fixture.hook_message(&scope, &info, "extension-mail", name, "frozen actual Copilot extension body");
    let mut historical = payload.clone();
    historical["session_id"] = serde_json::json!("historical-subagent");
    let before_historical = serde_json::to_value(fixture.hook_record(&scope, &info, name)).unwrap();
    for event in ["copilot-sessionstart", "copilot-posttooluse", "copilot-agentstop"] {
        let (code, _, stderr) = fixture.cli_input(scope.root(), &[event], &environment, Some(&historical), false);
        assert_eq!(code, 0, "{stderr}");
        assert_eq!(serde_json::to_value(fixture.hook_record(&scope, &info, name)).unwrap(), before_historical);
    }
    let (code, output, stderr) = fixture.cli_input(scope.root(), &["copilot-ext-poll", "--session-id", "copilot-main", "--idle"], &environment, None, false);
    assert_eq!(code, 0, "{stderr}");
    let offer: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(offer["event_id"], event);
    let receipt = serde_json::to_string(&offer["receipt"]).unwrap();
    let text = offer["text"].as_str().unwrap();
    assert!(text.contains("frozen actual Copilot extension body"));
    let (code, _, stderr) = fixture.cli_input(scope.root(), &["copilot-ext-sent", "--session-id", "copilot-main", "--receipt", &receipt], &environment, None, false);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(fixture.hook_record(&scope, &info, name).last_event_id, initial.last_event_id);
    let (_, output, _) = fixture.cli_input(scope.root(), &["copilot-posttooluse"], &environment, Some(&payload), false);
    assert_eq!(serde_json::from_str::<serde_json::Value>(&output).unwrap(), serde_json::json!({}), "ordinary hook stole extension-owned mail");
    prompt["prompt"] = serde_json::json!(format!("{text} plus unrelated text"));
    fixture.cli_input(scope.root(), &["copilot-userpromptsubmit"], &environment, Some(&prompt), false);
    assert_eq!(fixture.hook_record(&scope, &info, name).last_event_id, initial.last_event_id);
    prompt["prompt"] = serde_json::json!(text);
    fixture.cli_input(scope.root(), &["copilot-userpromptsubmit"], &environment, Some(&prompt), false);
    assert_eq!(fixture.hook_record(&scope, &info, name).last_event_id, event);
    let next = fixture.hook_message(&scope, &info, "hook-mail", name, "ordinary Copilot output after extension");
    fixture.cli_input(scope.root(), &["copilot-posttooluse"], &environment, Some(&payload), true);
    assert_eq!(fixture.hook_record(&scope, &info, name).last_event_id, event);
    let (code, output, stderr) = fixture.cli_input(scope.root(), &["copilot-posttooluse"], &environment, Some(&payload), false);
    assert_eq!(code, 0, "{stderr}");
    assert!(serde_json::from_str::<serde_json::Value>(&output).unwrap()["additionalContext"].as_str().unwrap().contains("ordinary Copilot output after extension"));
    assert_eq!(fixture.hook_record(&scope, &info, name).last_event_id, next);
    fixture.assert_daemon_descriptors(&scope, &info);
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux process execution"]
fn process_reset_failure_preserves_committed_state_and_generation() {
    // Fail both before the snapshot and inside the clear transaction. Fixture
    // SQL runs only while no daemon exists, never as a production client.
    for fail_clear in [false, true] {
        let fixture = Fixture::new();
        let scope = fixture.scope("scope");
        let path = scope.root().join("agentx.db");
        let generation = {
            let db = crate::db::AgentxDb::open_at(&path).unwrap();
            db.save_instance_named("lobo", serde_json::json!({"tool":"codex","created_at":10.0,"status":"active"}).as_object().unwrap()).unwrap();
            if fail_clear {
                db.conn().execute_batch("CREATE TRIGGER fixture_abort_clear BEFORE DELETE ON instances BEGIN SELECT RAISE(ABORT, 'fixture reset clear failure'); END;").unwrap();
            } else {
                fs::write(scope.root().join(crate::paths::ARCHIVE_DIR), "fixture archive path obstruction").unwrap();
            }
            db.database_generation().unwrap()
        };
        let _daemon = fixture.spawn_daemon(&scope);
        let info = fixture.ready(&scope);
        assert_eq!(generation, info.database_generation);
        let before = fixture.summary(&scope, &info);
        let result = fixture.call(&scope, fixture.caller(&info, "failed-reset"), Request::Maintenance(crate::service::maintenance::Request::ResetConversation)).unwrap();
        assert!(result.is_err(), "injected reset failure was ignored");
        let after = fixture.summary(&scope, &info);
        assert_eq!(before.local_participants, after.local_participants);
        assert_eq!(before.event_count, after.event_count);
        assert_eq!(before.database_generation, after.database_generation);
        assert_eq!(fixture.ready(&scope).database_generation, generation);
    }
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and AGENTX_ACCEPTANCE_BASELINE; controlled mixed-version fixture"]
fn process_legacy_cutover_requires_retirement_before_candidate_exclusivity() {
    let fixture = Fixture::new();
    let baseline = fs::canonicalize(std::env::var_os("AGENTX_ACCEPTANCE_BASELINE").expect("set preserved legacy executable")).unwrap();
    assert_ne!(baseline, fixture.binary);
    let scope = fixture.scope("scope");
    let start = fixture.command(scope.root());
    // Preserve the same hermetic env and cwd while choosing old bytes; Command
    // has no set_program, so copy only the explicit fixture variables.
    let mut old_start = Command::new(&baseline);
    old_start.env_clear().envs(start.get_envs().filter_map(|(key, value)| value.map(|value| (key, value)))).current_dir(start.get_current_dir().unwrap()).stdin(Stdio::null()).args(["start"]);
    let output = old_start.output().unwrap();
    assert!(output.status.success(), "legacy fixture start: {}", String::from_utf8_lossy(&output.stderr));
    let text = String::from_utf8(output.stdout).unwrap();
    let name = text.lines().find_map(|line| line.trim_start().strip_prefix("[agentx:").and_then(|tail| tail.split(']').next())).expect("legacy participant marker");
    let mut old_listen = Command::new(&baseline);
    old_listen.env_clear().envs(start.get_envs().filter_map(|(key, value)| value.map(|value| (key, value)))).current_dir(start.get_current_dir().unwrap()).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).args([
        "listen",
        "--name",
        name,
        "--timeout",
        "600",
        "--json",
    ]);
    let mut legacy = OwnedDaemon(old_listen.spawn().unwrap());
    let database = scope.root().join("agentx.db");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let (handles, _) = managed_descriptors(&database);
        if handles.iter().any(|(pid, _)| *pid == legacy.id()) {
            break;
        }
        assert!(Instant::now() < deadline, "legacy listener never opened managed DB");
        std::thread::sleep(Duration::from_millis(10));
    }
    let _candidate = fixture.spawn_daemon(&scope);
    let info = fixture.ready(&scope);
    let (mixed_handles, denied) = managed_descriptors(&database);
    println!("DD-08 expected unsupported mixed deployment: {mixed_handles:?}; inaccessible={denied:?}");
    assert!(mixed_handles.iter().any(|(pid, _)| *pid == legacy.id()), "old client did not demonstrate cutover precondition");
    assert!(mixed_handles.iter().any(|(pid, _)| *pid == info.pid));
    let legacy_pid = legacy.id();
    legacy.kill().unwrap();
    legacy.wait().unwrap();
    let (retired_handles, denied) = managed_descriptors(&database);
    assert!(!retired_handles.iter().any(|(pid, _)| *pid == legacy_pid));
    assert!(retired_handles.iter().all(|(pid, _)| *pid == info.pid), "candidate-only supported deployment: {retired_handles:?}; inaccessible={denied:?}");
    let (code, _, stderr) = fixture.cli(scope.root(), &["list", "--json"]);
    assert_eq!(code, 0, "candidate client after cutover: {stderr}");
}

#[test]
#[ignore = "requires fixed AGENTX_ACCEPTANCE_BINARY and disposable Linux process execution"]
fn process_codex_model_request_error_lifecycle() {
    let fixture = Fixture::new();
    let scope = fixture.scope("model-errors");
    let _daemon = fixture.spawn_daemon(&scope);
    let info = fixture.ready(&scope);
    let binding = "fixture-model-errors";
    let participant = fixture.register_hook(&scope, &info, "codex", binding);
    let name = participant.canonical_name.as_str();
    let environment = [("AGENTX_PROCESS_ID", binding), ("AGENTX_TOOL", "codex"), ("AGENTX_LAUNCHED", "1")];
    let mut payload = serde_json::json!({"session_id":"model-session", "turn_id":"turn-1", "model":"fixture", "source":"startup", "cwd":fixture.root.path().join("workspace")});
    let (code, _, stderr) = fixture.cli_input(scope.root(), &["codex-sessionstart"], &environment, Some(&payload), false);
    assert_eq!(code, 0, "{stderr}");
    for (command, status, expected) in
        [("codex-premodelrequest", "", "active"), ("codex-postmodelrequest", "failed", "error"), ("codex-stop", "", "error"), ("codex-premodelrequest", "", "error"), ("codex-postmodelrequest", "completed", "active"), ("codex-stop", "", "listening")]
    {
        payload["status"] = serde_json::json!(status);
        payload["error"] = serde_json::json!("upstream 503");
        let (code, _, stderr) = fixture.cli_input(scope.root(), &[command], &environment, Some(&payload), false);
        assert_eq!(code, 0, "{command}: {stderr}");
        let row = fixture.hook_record(&scope, &info, name);
        assert_eq!(row.status, expected, "{command}: {stderr}");
        if expected == "error" {
            assert_eq!(row.model_request.as_ref().unwrap().error.as_ref().unwrap().reason, "upstream 503");
            let (code, output, stderr) = fixture.cli(scope.root(), &["list", name, "--json"]);
            assert_eq!(code, 0, "{stderr}");
            let value: serde_json::Value = serde_json::from_str(&output).unwrap();
            assert!(output.contains("model_request") && output.contains("upstream 503"), "{value}");
        }
    }
}
