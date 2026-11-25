//! Independent OBS-01/02/06/07/08/10 native integration through real listeners.
//! The Application wrapper only gates scheduling; real SQLite handles requests.
//! Executions/limits belong to the original native-integration Task and receipt.

#![cfg(target_os = "linux")]

use std::path::Path;
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::observation::{self, InspectionRequest, InspectionResponse};
use super::wire::{ClientFrame, ServerFrame};
use super::{DaemonError, DaemonScope};
use crate::observability::configuration::ConfigurationRequest;
use crate::observability::context::{DaemonRef, EvidenceValue, ScopeRef};
use crate::observability::events::{ObservationCursor, ObservationFilter, ObservationHead, ObservationPayload, ObservationRead, ObservationReadResult, ObservationRequest, ObservationResponse, ObservationWindow};
use crate::observability::runtime::{DispatchActivity, ExecutionObservation, ExistingScopeInput, LocalFailureKind, NotRunReason, RequestSelection, RuntimeFailureCode, RuntimeList, RuntimeRequest, RuntimeResponse, RuntimeSnapshot, ScopeLookup};
use crate::service::{Application, CallerContext, InvocationContext, Request, RequestContext, RequestId, Response, ServiceResult};

const BOUND: Duration = Duration::from_secs(5);

/// Exact direct children only; panic cleanup cannot leave a follow process.
struct ObserverChild(std::process::Child);
impl ObserverChild {
    fn wait(&mut self) -> std::process::ExitStatus {
        let deadline = Instant::now() + BOUND;
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "actual observer failed to stop within its bounded lifetime");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for ObserverChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Passive reads may encounter two genuine response tails after receiving a
/// previous Result. Retry only the known capacity close class, within one
/// finite window; owner responses and epoch/peer/protocol errors are never retried.
fn read_with_capacity_retry<T>(deadline: Instant, mut read: impl FnMut() -> Result<T, crate::observability::runtime::LocalFailure>) -> Result<T, crate::observability::runtime::LocalFailure> {
    loop {
        match read() {
            Err(error) if is_capacity_close(&error) && Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            result => return result,
        }
    }
}

fn is_capacity_close(error: &crate::observability::runtime::LocalFailure) -> bool {
    use crate::observability::runtime::ClientStage;
    matches!(error.stage, ClientStage::InspectionWrite | ClientStage::InspectionRead) && error.kind == LocalFailureKind::Io && matches!(error.os_code, Some(nix::libc::ECONNRESET | nix::libc::ECONNABORTED | nix::libc::EPIPE))
}

#[test]
fn passive_read_retry_is_bounded_and_does_not_hide_authority_failures() {
    use crate::observability::runtime::{ClientStage, LocalFailure};
    let reset = LocalFailure { stage: ClientStage::InspectionWrite, kind: LocalFailureKind::Io, os_code: Some(nix::libc::ECONNRESET), at: None };
    let mut attempts = 0;
    let recovered = read_with_capacity_retry(Instant::now() + BOUND, || {
        attempts += 1;
        if attempts == 1 { Err(reset.clone()) } else { Ok(7) }
    })
    .unwrap();
    assert_eq!((recovered, attempts), (7, 2));
    for failure in [
        LocalFailure { stage: ClientStage::Connect, ..reset.clone() },
        LocalFailure { kind: LocalFailureKind::StaleGeneration, ..reset.clone() },
        LocalFailure { kind: LocalFailureKind::Incompatible, ..reset.clone() },
        LocalFailure { stage: ClientStage::PeerAuthentication, kind: LocalFailureKind::PeerRejected, ..reset.clone() },
        LocalFailure { kind: LocalFailureKind::Protocol, ..reset.clone() },
        LocalFailure { os_code: None, ..reset.clone() },
        LocalFailure { os_code: Some(nix::libc::EACCES), ..reset.clone() },
    ] {
        let mut attempts = 0;
        let returned = read_with_capacity_retry(Instant::now() + BOUND, || {
            attempts += 1;
            Err::<(), _>(failure.clone())
        })
        .unwrap_err();
        assert_eq!(returned, failure);
        assert_eq!(attempts, 1);
    }
    let mut attempts = 0;
    assert_eq!(
        read_with_capacity_retry(Instant::now(), || {
            attempts += 1;
            Err::<(), _>(reset.clone())
        })
        .unwrap_err(),
        reset
    );
    assert_eq!(attempts, 1, "expired capacity retry must return the original failure");
}

#[derive(Default)]
struct DispatchGate {
    held: Mutex<bool>,
    released: Condvar,
}

impl DispatchGate {
    fn release(&self) {
        *self.held.lock().unwrap() = false;
        self.released.notify_all();
    }
}

struct GatedApplication {
    inner: crate::service::application::SqliteApplication,
    gate: Arc<DispatchGate>,
    entered: mpsc::SyncSender<()>,
    calls: Arc<AtomicUsize>,
}

impl Application for GatedApplication {
    fn handle(&mut self, context: &RequestContext, request: Request) -> ServiceResult<Response> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        {
            let held = self.gate.held.lock().unwrap();
            if *held {
                let _ = self.entered.try_send(());
                let (held, timeout) = self.gate.released.wait_timeout_while(held, Duration::from_secs(20), |held| *held).unwrap();
                assert!(!timeout.timed_out() || !*held, "independent fixture dispatcher gate expired");
            }
        }
        self.inner.handle(context, request)
    }
}

struct Fixture {
    directory: tempfile::TempDir,
    scope: DaemonScope,
    reference: ScopeRef,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<(), DaemonError>>>,
    gate: Arc<DispatchGate>,
    entered: mpsc::Receiver<()>,
    calls: Arc<AtomicUsize>,
    initial_generation: String,
}

impl Fixture {
    fn start(hold_dispatch: bool) -> Self {
        // A long inherited TMPDIR can itself exceed the 2KiB request-detail
        // projection budget through repeated scope refs. This clean scenario
        // owns a short explicit root; pressure/oversize has separate cases.
        let directory = tempfile::Builder::new().prefix("ax-obs-").tempdir_in("/tmp").unwrap();
        crate::paths::test_roots::register(directory.path());
        let scope = DaemonScope::prepare(directory.path()).unwrap();
        let reference = ScopeRef::from_canonical_scope(scope.root().to_path_buf()).unwrap();
        let database = directory.path().join("agentx.db");
        let stop = Arc::new(AtomicBool::new(false));
        let gate = Arc::new(DispatchGate { held: Mutex::new(hold_dispatch), released: Condvar::new() });
        let (sender, entered) = mpsc::sync_channel(1);
        let calls = Arc::new(AtomicUsize::new(0));
        let thread_scope = scope.clone();
        let thread_stop = stop.clone();
        let thread_gate = gate.clone();
        let thread_calls = calls.clone();
        let thread = std::thread::spawn(move || {
            // Exact worker/queue assertions require only this fixture's callers;
            // management's Inspector poller independently submits business reads.
            super::serve_with_stop_and_management(thread_scope, thread_stop, false, |generation| {
                let db = crate::db::AgentxDb::open_at(&database).map_err(|error| crate::service::ServiceError::Internal(error.to_string()))?;
                let inner = crate::service::application::SqliteApplication::new(db, generation)?;
                Ok(GatedApplication { inner, gate: thread_gate, entered: sender, calls: thread_calls })
            })
        });
        let mut fixture = Self { directory, scope, reference, stop, thread: Some(thread), gate, entered, calls, initial_generation: String::new() };
        fixture.wait_snapshot(|value| matches!(value.generation.value, EvidenceValue::Present(_)));
        let deadline = Instant::now() + BOUND;
        fixture.initial_generation = loop {
            if let Ok(info) = super::probe(&fixture.scope) {
                break info.database_generation;
            }
            assert!(Instant::now() < deadline, "ordinary listener never became ready");
            std::thread::sleep(Duration::from_millis(5));
        };
        fixture.wait_snapshot(|value| value.gauges.business_connections == 0);
        fixture
    }

    fn inspect(&self, expected: Option<DaemonRef>, input: InspectionRequest) -> super::observation::InspectionResult {
        read_with_capacity_retry(Instant::now() + BOUND, || observation::inspect_existing(&self.reference, expected.clone(), input.clone())).unwrap()
    }

    fn snapshot(&self) -> RuntimeSnapshot {
        self.wait_snapshot(|_| true)
    }

    fn wait_snapshot(&self, predicate: impl Fn(&RuntimeSnapshot) -> bool) -> RuntimeSnapshot {
        let deadline = Instant::now() + BOUND;
        loop {
            let result = observation::inspect_existing(&self.reference, None, InspectionRequest::Runtime(RuntimeRequest::Snapshot));
            if let Ok(result) = result {
                match result.response {
                    InspectionResponse::Runtime(Ok(RuntimeResponse::Snapshot(value))) if predicate(&value) => return value,
                    InspectionResponse::Runtime(Ok(_)) | InspectionResponse::Runtime(Err(crate::observability::runtime::RuntimeReadError::Busy | crate::observability::runtime::RuntimeReadError::Unavailable)) => {}
                    other => panic!("unexpected diagnostic readiness branch: {other:?}"),
                }
            }
            assert!(!self.thread.as_ref().unwrap().is_finished(), "fixture daemon exited before observation");
            assert!(Instant::now() < deadline, "diagnostic readiness/state deadline for {}", self.directory.path().display());
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn connect_business(&self) -> super::platform::Stream {
        super::platform::connect(&self.scope, Instant::now() + BOUND).unwrap()
    }

    fn send_watermark(&self, id: &str) -> super::platform::Stream {
        self.send(id, InvocationContext::default(), Request::Events(crate::events::service::Request::Watermark))
    }

    fn send(&self, id: &str, mut invocation: InvocationContext, request: Request) -> super::platform::Stream {
        let mut stream = self.connect_business();
        invocation.cwd = self.directory.path().to_path_buf();
        let caller = CallerContext { request_id: RequestId(id.into()), invocation, deadline_unix_ms: None, expected_database_generation: Some(self.initial_generation.clone()) };
        super::frame::write(&mut stream, &ClientFrame::Call { canonical_scope: self.scope.root().into(), daemon_instance: None, caller, request }, Instant::now() + BOUND).unwrap();
        stream
    }

    fn call(&self, id: &str, invocation: InvocationContext, request: Request) -> ServiceResult<Response> {
        let mut stream = self.send(id, invocation, request);
        match super::frame::read(&mut stream, Instant::now() + BOUND).unwrap() {
            ServerFrame::Result { result, .. } => result,
            other => panic!("native call rejected: {}", serde_json::to_string(&other).unwrap()),
        }
    }

    fn head(&self) -> ObservationHead {
        match self.inspect(None, InspectionRequest::Events(ObservationRequest::Describe)).response {
            InspectionResponse::Events(Ok(ObservationResponse::Head(head))) => head,
            other => panic!("stream head returned {other:?}"),
        }
    }

    fn request(&self, id: &str) -> crate::observability::runtime::RequestObservation {
        let deadline = Instant::now() + BOUND;
        loop {
            let input = RuntimeRequest::Requests { select: RequestSelection::LogicalRequest(RequestId(id.into())), page: RuntimeList { before: None, limit: 8 } };
            match self.inspect(None, InspectionRequest::Runtime(input)).response {
                InspectionResponse::Runtime(Ok(RuntimeResponse::Requests(mut page))) => {
                    assert_eq!(page.rows.len(), 1, "logical fixture request must have one retained actual decode; clean short-root fixture must not pressure the detail budget");
                    return page.rows.remove(0);
                }
                InspectionResponse::Runtime(Err(crate::observability::runtime::RuntimeReadError::Busy | crate::observability::runtime::RuntimeReadError::Unavailable)) => {}
                other => panic!("logical request read returned {other:?}"),
            }
            assert!(Instant::now() < deadline, "request detail stayed unavailable within the bounded read window");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn observer_command(&self, binary: &Path) -> std::process::Command {
        let home = self.directory.path().join("observer-home");
        std::fs::create_dir_all(&home).unwrap();
        let mut command = std::process::Command::new(binary);
        command
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LANG", "C.UTF-8")
            .env("HOME", &home)
            .env("AGENTX_DIR", self.scope.root())
            .env("CI", "1")
            .env("NO_COLOR", "1")
            .env("AGENTX_UPDATE_VERIFY", "1")
            .env("XDG_CONFIG_HOME", home.join("config"))
            .env("XDG_CACHE_HOME", home.join("cache"))
            .env("XDG_DATA_HOME", home.join("data"))
            .env("CODEX_HOME", home.join("codex"))
            .env("CLAUDE_CONFIG_DIR", home.join("claude"))
            .env("TMPDIR", self.directory.path())
            .current_dir(self.directory.path())
            .stdin(std::process::Stdio::null());
        command
    }
}

#[test]
#[ignore = "requires fixed same-cohort AGENTX_ACCEPTANCE_BINARY and AGENTX_OBSERVATION_EVIDENCE"]
fn actual_cli_inspects_held_work_and_interrupts_only_its_own_follow() {
    use std::io::BufRead;
    use std::process::Stdio;
    let binary = std::fs::canonicalize(std::env::var_os("AGENTX_ACCEPTANCE_BINARY").expect("select the fixed source-built observer")).unwrap();
    let image = std::fs::read(&binary).unwrap();
    let stamp = super::wire::build_identity().as_bytes();
    assert!(image.windows(stamp.len()).any(|bytes| bytes == stamp), "product and fixture must share Cargo's native build cohort; do not override compatibility");
    let evidence = std::path::PathBuf::from(std::env::var_os("AGENTX_OBSERVATION_EVIDENCE").expect("select exact retained observer evidence"));
    std::fs::create_dir(&evidence).unwrap();
    let fixture = Fixture::start(true);
    let mut running = fixture.send_watermark("obs-actual-cli-held");
    fixture.entered.recv_timeout(BOUND).unwrap();
    let stdout = evidence.join("inspect.stdout");
    let stderr = evidence.join("inspect.stderr");
    let mut child = ObserverChild(fixture.observer_command(&binary).args(["observe", "inspect", "--json"]).stdout(std::fs::File::create(&stdout).unwrap()).stderr(std::fs::File::create(stderr).unwrap()).spawn().unwrap());
    assert_eq!(child.wait().code(), Some(0));
    let record: serde_json::Value = serde_json::from_slice(&std::fs::read(stdout).unwrap()).unwrap();
    let inspected: observation::InspectionResult = serde_json::from_value(record["result"]["Inspection"].clone()).unwrap();
    let InspectionResponse::Runtime(Ok(RuntimeResponse::Snapshot(snapshot))) = inspected.response else {
        panic!("actual inspect lost native snapshot");
    };
    let DispatchActivity::Running(reference) = snapshot.activity else {
        panic!("actual inspect lost held work");
    };
    assert_eq!(snapshot.gauges.executing_jobs, 1);
    let request_file = evidence.join("request.json");
    std::fs::write(&request_file, serde_json::to_vec(&reference).unwrap()).unwrap();
    let stdout = evidence.join("request.stdout");
    let mut child = ObserverChild(
        fixture
            .observer_command(&binary)
            .args(["observe", "request", "--json", "--ref-file"])
            .arg(&request_file)
            .stdout(std::fs::File::create(&stdout).unwrap())
            .stderr(std::fs::File::create(evidence.join("request.stderr")).unwrap())
            .spawn()
            .unwrap(),
    );
    assert_eq!(child.wait().code(), Some(0));
    let record: serde_json::Value = serde_json::from_slice(&std::fs::read(stdout).unwrap()).unwrap();
    let inspected: observation::InspectionResult = serde_json::from_value(record["result"]["Inspection"].clone()).unwrap();
    assert!(matches!(inspected.response, InspectionResponse::Runtime(Ok(RuntimeResponse::RequestDetail(value))) if matches!(value.execution, ExecutionObservation::Running { .. })));
    for interrupt in [true, false] {
        let label = if interrupt { "sigint" } else { "broken-pipe" };
        let mut observer = ObserverChild(fixture.observer_command(&binary).args(["observe", "events", "--follow", "--json"]).stdout(Stdio::piped()).stderr(std::fs::File::create(evidence.join(format!("{label}.stderr"))).unwrap()).spawn().unwrap());
        let output = observer.0.stdout.take().unwrap();
        let (sender, receiver) = mpsc::sync_channel(1);
        let (release, hold_reader) = mpsc::sync_channel(1);
        let reader = std::thread::spawn(move || {
            let mut output = std::io::BufReader::new(output);
            let mut line = String::new();
            let result = output.read_line(&mut line).map(|_| line);
            let _ = sender.send(result);
            // Keep the read end open for SIGINT, so it cannot become a
            // coincident BrokenPipe that masks the signal's actual exit.
            if interrupt {
                let _ = hold_reader.recv_timeout(BOUND);
            }
        });
        let line = receiver.recv_timeout(BOUND).unwrap().unwrap();
        std::fs::write(evidence.join(format!("{label}.first.stdout")), &line).unwrap();
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert!(value["result"]["Inspection"]["response"]["Events"]["Ok"]["Read"]["Page"].is_object());
        let pid = observer.0.id();
        if interrupt {
            // SAFETY: this exact positive PID belongs to our unreaped child.
            assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGINT) }, 0);
        }
        let status = observer.wait();
        let _ = release.send(());
        reader.join().unwrap();
        assert_eq!(status.code(), Some(if interrupt { 130 } else { 0 }));
        std::fs::write(evidence.join(format!("{label}.exit.json")), format!("{{\"pid\":{pid},\"code\":{}}}\n", status.code().unwrap())).unwrap();
        assert!(!fixture.thread.as_ref().unwrap().is_finished(), "observer stopped the daemon");
        assert_eq!(fixture.calls.load(Ordering::SeqCst), 1, "observer dispatched business work");
        assert!(matches!(fixture.request("obs-actual-cli-held").execution, ExecutionObservation::Running { .. }));
    }
    fixture.gate.release();
    assert!(matches!(super::frame::read(&mut running, Instant::now() + BOUND).unwrap(), ServerFrame::Result { result: Ok(Response::Events(_)), .. }));
    fixture.wait_snapshot(|value| value.gauges.executing_jobs == 0);
    assert!(matches!(fixture.request("obs-actual-cli-held").execution, ExecutionObservation::Returned { .. }));
    println!("actual observer={} cohort={} held-work completed; sigint=130 broken-pipe=0 evidence={}", binary.display(), super::wire::build_identity(), evidence.display());
}

#[test]
#[ignore = "requires fixed same-cohort AGENTX_ACCEPTANCE_BINARY and AGENTX_OBSERVATION_EVIDENCE"]
fn actual_cli_compares_application_captures_from_two_originating_environments() {
    let binary = std::fs::canonicalize(std::env::var_os("AGENTX_ACCEPTANCE_BINARY").expect("select fixed same-cohort CLI")).unwrap();
    let image = std::fs::read(&binary).unwrap();
    let stamp = super::wire::build_identity().as_bytes();
    assert!(image.windows(stamp.len()).any(|bytes| bytes == stamp));
    let evidence = std::path::PathBuf::from(std::env::var_os("AGENTX_OBSERVATION_EVIDENCE").expect("select exact configuration evidence"));
    std::fs::create_dir(&evidence).unwrap();
    let fixture = Fixture::start(false);
    // CLI-03: real application resolutions from two originating environments,
    // then the actual diagnostic executable consumes their native references.
    use crate::agents::service::{Registration, Request as Participants, Response as ParticipantResponse};
    use crate::messaging::service::{InboxOperation, Request as Messages};
    use crate::observability::configuration::{ConfigObservationEvent, ConfigStage, InvocationCaptureHandle};
    use crate::observability::runtime::{ClientAttemptId, ClientInvocationId, OriginRef};
    let binding = uuid::Uuid::new_v4().to_string();
    let registration = Registration { name: None, tool: "qoder".into(), session_id: None, process_binding_id: Some(binding.clone()), transcript_path: None, wait_timeout: 30, subagent_timeout: None, hints: None, auto_subscribe: String::new() };
    let participant = match fixture.call("obs-cli-config-register", InvocationContext { process_binding_id: Some(binding), ..Default::default() }, Request::Participants(Participants::Register(registration))).unwrap() {
        Response::Participants(ParticipantResponse::Registered(actor)) => actor.participant,
        other => panic!("configuration fixture registration returned {other:?}"),
    };
    let mut references = Vec::new();
    for (label, timeout) in [("left", "17"), ("right", "29")] {
        let id = format!("obs-cli-config-{label}");
        let origin = OriginRef { invocation: InvocationCaptureHandle::begin(ClientInvocationId::new()).reference(), attempt: ClientAttemptId::new() };
        let attempt = origin.attempt;
        let before = fixture.snapshot().coverage.request_details_dropped;
        let invocation = InvocationContext {
            environment: std::collections::HashMap::from([("AGENTX_TIMEOUT".into(), timeout.into()), ("AGENTX_RELAY_TOKEN".into(), "private-config-sentinel".into()), ("AGENTX_TERMINAL".into(), "private-terminal-command -e {script}".into())]),
            observation_origin: Some(origin),
            ..Default::default()
        };
        assert!(matches!(fixture.call(&id, invocation, Request::Messages(Messages::Inbox(InboxOperation::Snapshot { participant: participant.clone() }))).unwrap(), Response::Messages(_)));
        // A complete detail can exceed the native 2KiB budget even here.
        // Observe its explicit loss; obtain config identity only from exact
        // same-attempt publication facts, never from value/time/sequence joins.
        let after = fixture.snapshot();
        std::fs::write(evidence.join(format!("{label}-runtime.json")), serde_json::to_vec(&after).unwrap()).unwrap();
        let input = RuntimeRequest::Requests { select: RequestSelection::LogicalRequest(RequestId(id.clone())), page: RuntimeList { before: None, limit: 8 } };
        match fixture.inspect(None, InspectionRequest::Runtime(input)).response {
            InspectionResponse::Runtime(Ok(RuntimeResponse::Requests(page))) => {
                if page.rows.is_empty() {
                    assert!(after.coverage.request_details_dropped > before, "missing detail requires explicit dropped coverage");
                }
            }
            other => panic!("actual request selection failed: {other:?}"),
        }
        let head = fixture.head();
        let window = ObservationWindow { after: ObservationCursor { stream: head.stream, sequence: 0 }, through: head.newest, filter: ObservationFilter { client_attempt: Some(attempt), ..Default::default() } };
        let response = fixture.inspect(None, InspectionRequest::Events(ObservationRequest::Read(ObservationRead { window, limit: 256 }))).response;
        std::fs::write(evidence.join(format!("{label}-config-events.json")), serde_json::to_vec(&response).unwrap()).unwrap();
        let InspectionResponse::Events(Ok(ObservationResponse::Read(ObservationReadResult::Page(page)))) = response else { panic!("exact configuration publication evidence unavailable") };
        let mut refs = page.records.into_iter().filter_map(|row| match row.input.payload {
            ObservationPayload::Configuration(ConfigObservationEvent::SnapshotPublished { snapshot, attempt: Some(observed), stage: ConfigStage::ScopedEffective, .. }) if observed == attempt => Some(snapshot),
            _ => None,
        });
        let reference = refs.next().expect("actual same-attempt effective capture must be retained");
        assert!(refs.next().is_none(), "ambiguous configuration publication identity");
        assert_eq!(reference.epoch.daemon.scope, fixture.reference);
        let file = evidence.join(format!("config-{label}.json"));
        std::fs::write(&file, serde_json::to_vec(&reference).unwrap()).unwrap();
        references.push(file);
    }
    let cli_json = |label: &str, args: &[&str]| -> serde_json::Value {
        let output = fixture.observer_command(&binary).args(args).output().unwrap();
        std::fs::write(evidence.join(format!("{label}.stdout")), &output.stdout).unwrap();
        std::fs::write(evidence.join(format!("{label}.stderr")), &output.stderr).unwrap();
        std::fs::write(evidence.join(format!("{label}.exit.json")), format!("{{\"code\":{:?}}}\n", output.status.code().unwrap())).unwrap();
        assert!(output.status.success(), "actual CLI {label} failed: {:?}", output.status);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("private-config-sentinel"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("private-terminal-command"));
        serde_json::from_slice(&output.stdout).unwrap()
    };
    let left = cli_json("config-left", &["observe", "config", "--json", "--ref-file", references[0].to_str().unwrap()]);
    let right = cli_json("config-right", &["observe", "config", "--json", "--ref-file", references[1].to_str().unwrap()]);
    for (value, seconds) in [(&left, 17), (&right, 29)] {
        let snapshot = &value["result"]["Inspection"]["response"]["Configuration"]["Ok"]["Snapshot"]["result"]["Found"];
        assert_eq!(snapshot["body"]["purpose"], "ActualResolution");
        assert_eq!(snapshot["provenance"], "ApplicationResolved");
        let fields = snapshot["body"]["fields"].as_array().unwrap();
        let timeout = fields.iter().find(|field| field["field"]["Setting"] == "timeout").unwrap();
        assert_eq!(timeout["value"], serde_json::json!({"Present":{"Seconds": seconds}}));
    }
    let compared = cli_json("config-compare", &["observe", "compare", "--json", "--left-file", references[0].to_str().unwrap(), "--right-file", references[1].to_str().unwrap(), "--field", "timeout", "--field", "terminal"]);
    let fields = compared["result"]["Inspection"]["response"]["Configuration"]["Ok"]["Comparison"]["result"]["Compared"]["fields"].as_array().unwrap();
    assert_eq!(fields.iter().find(|field| field["field"]["Setting"] == "timeout").unwrap()["value"], "Different");
    assert_eq!(fields.iter().find(|field| field["field"]["Setting"] == "terminal").unwrap()["value"], serde_json::json!({"Unknown":"Withheld"}));
    std::fs::write(fixture.directory.path().join("config.toml"), "[preferences]\ntimeout = 41\n").unwrap();
    let current = cli_json("config-current", &["observe", "config-current", "--json"]);
    assert_eq!(current["result"]["ConfigurationCurrent"]["Ok"]["body"]["purpose"], "DiagnosticReevaluation");
    let fields = current["result"]["ConfigurationCurrent"]["Ok"]["body"]["fields"].as_array().unwrap();
    assert_eq!(fields.iter().find(|field| field["field"]["Setting"] == "timeout").unwrap()["value"], serde_json::json!({"Present":{"Seconds":41}}));
    let retained = cli_json("config-left-after-edit", &["observe", "config", "--json", "--ref-file", references[0].to_str().unwrap()]);
    assert_eq!(
        left["result"]["Inspection"]["response"]["Configuration"]["Ok"]["Snapshot"]["result"]["Found"], retained["result"]["Inspection"]["response"]["Configuration"]["Ok"]["Snapshot"]["result"]["Found"],
        "historical configuration changed after a file edit"
    );
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.gate.release();
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let deadline = Instant::now() + Duration::from_secs(35);
            while !thread.is_finished() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
            if !thread.is_finished() {
                eprintln!("fixture daemon cleanup exceeded deadline: {}", self.directory.path().display());
                assert!(std::thread::panicking(), "fixture daemon did not stop");
                return;
            }
            let result = thread.join();
            if !std::thread::panicking() {
                result.unwrap().unwrap();
                assert!(!self.scope.observation_socket_path(false).unwrap().exists(), "observer endpoint survived its scope lease");
            }
        }
    }
}

#[test]
fn passive_lookup_and_failure_leave_absent_and_existing_scopes_untouched() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::Builder::new().prefix("agentx-obs-no-effects-").tempdir().unwrap();
    let missing = directory.path().join("missing/child");
    let input = ExistingScopeInput { requested: missing, cwd: directory.path().to_path_buf() };
    assert!(matches!(observation::resolve_existing_scope(input), ScopeLookup::Missing));
    assert!(!directory.path().join("missing").exists());
    let existing = directory.path().join("existing");
    std::fs::create_dir(&existing).unwrap();
    std::fs::set_permissions(&existing, std::fs::Permissions::from_mode(0o700)).unwrap();
    let scope = ScopeRef::from_canonical_scope(existing.clone()).unwrap();
    let error = observation::inspect_existing(&scope, None, InspectionRequest::Runtime(RuntimeRequest::Snapshot)).unwrap_err();
    assert!(matches!(error.kind, LocalFailureKind::EndpointMissing | LocalFailureKind::ConnectionRefused));
    assert_eq!(std::fs::read_dir(&existing).unwrap().count(), 0, "diagnosis constructed state/managed storage");
    assert_eq!(std::fs::metadata(existing).unwrap().permissions().mode() & 0o777, 0o700);
}

#[test]
fn inspection_remains_callable_with_all_thirty_two_ordinary_workers_occupied() {
    let fixture = Fixture::start(false);
    let mut occupied = Vec::new();
    for _ in 0..32 {
        occupied.push(fixture.connect_business());
    }
    let started = Instant::now();
    let snapshot = fixture.wait_snapshot(|value| value.gauges.business_connections == 32);
    assert!(started.elapsed() < Duration::from_millis(1_500), "passive lane exceeded the one-second contract plus harness scheduling margin");
    assert_eq!(snapshot.gauges.business_connections, 32);
    assert_eq!(snapshot.gauges.executing_jobs, 0);
    assert!(snapshot.gauges.inspection_connections <= 2);
    let ready = fixture.inspect(None, InspectionRequest::Events(ObservationRequest::Describe)).ready;
    assert_eq!(ready.process.pid, std::process::id(), "Ready process must match the authenticated real listener");
    assert_eq!(ready.process.incarnation, crate::sys::process::identity(std::process::id()));
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 0, "inspection invoked the real Application");
    drop(occupied);
    fixture.wait_snapshot(|value| value.gauges.business_connections == 0);
}

#[test]
fn inspection_distinguishes_running_and_queued_requests_while_application_is_held() {
    let fixture = Fixture::start(true);
    let mut running = fixture.send_watermark("obs-running");
    fixture.entered.recv_timeout(BOUND).unwrap();
    let mut queued = Vec::new();
    for index in 0..16 {
        queued.push(fixture.send_watermark(&format!("obs-queued-{index}")));
    }
    fixture.wait_snapshot(|value| value.gauges.executing_jobs == 1 && value.gauges.queued_jobs == 16);
    let mut overflow = fixture.send_watermark("obs-queue-overflow");
    assert!(matches!(super::frame::read(&mut overflow, Instant::now() + BOUND).unwrap(), ServerFrame::Failure(DaemonError::Busy)));
    drop(overflow);
    fixture.wait_snapshot(|value| value.gauges.business_connections == 17);
    let mut occupied = Vec::new();
    for _ in 0..15 {
        occupied.push(fixture.connect_business());
    }
    let started = Instant::now();
    let snapshot = fixture.wait_snapshot(|value| value.gauges.business_connections == 32 && value.gauges.executing_jobs == 1 && value.gauges.queued_jobs == 16);
    assert!(started.elapsed() < Duration::from_millis(1_500), "full ordinary admission/queue blocked passive inspection");
    assert!(snapshot.gauges.inspection_connections <= 2);
    assert!(matches!(snapshot.activity, DispatchActivity::Running(_)));
    let first = fixture.request("obs-running");
    let second = fixture.request("obs-queued-0");
    let last = fixture.request("obs-queued-15");
    assert!(matches!(first.execution, ExecutionObservation::Running { .. }));
    assert!(matches!(second.execution, ExecutionObservation::Queued { .. }));
    assert!(matches!(last.execution, ExecutionObservation::Queued { .. }));
    assert!(second.enqueued_at.is_some() && second.dequeued_at.is_none());
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 1, "inspection bypassed the held dispatcher");
    drop(occupied);
    fixture.gate.release();
    for stream in std::iter::once(&mut running).chain(queued.iter_mut()) {
        assert!(matches!(super::frame::read(stream, Instant::now() + BOUND).unwrap(), ServerFrame::Result { result: Ok(Response::Events(_)), .. }));
    }
    fixture.wait_snapshot(|value| value.gauges.executing_jobs == 0 && value.gauges.queued_jobs == 0);
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 17, "overflow rejection or inspection entered Application");
    assert!(matches!(fixture.request("obs-running").execution, ExecutionObservation::Returned { .. }));
    assert!(matches!(fixture.request("obs-queued-15").execution, ExecutionObservation::Returned { .. }));
}

#[test]
fn stale_expected_daemon_is_rejected_before_every_owner_leaf() {
    let fixture = Fixture::start(false);
    let actual = fixture.snapshot().daemon;
    let mut stale = actual.clone();
    stale.daemon_instance = "fixture-retired-daemon".into();
    let requests = [InspectionRequest::Runtime(RuntimeRequest::Snapshot), InspectionRequest::Configuration(ConfigurationRequest::Head { expected: None }), InspectionRequest::Events(ObservationRequest::Describe)];
    for request in requests {
        match fixture.inspect(Some(stale.clone()), request).response {
            InspectionResponse::DaemonChanged { expected, actual: returned } => {
                assert_eq!(expected, stale);
                assert_eq!(returned, actual);
            }
            other => panic!("stale daemon selection reached owner routing: {other:?}"),
        }
    }
}

#[test]
fn separate_scopes_have_distinct_streams_and_passive_aliases_resolve_identically() {
    let first = Fixture::start(false);
    let second = Fixture::start(false);
    let alias = first.directory.path().join("alias");
    std::os::unix::fs::symlink(first.scope.root(), &alias).unwrap();
    let lookup = observation::resolve_existing_scope(ExistingScopeInput { requested: alias, cwd: Path::new("/").to_path_buf() });
    assert!(matches!(lookup, ScopeLookup::Existing(value) if value == first.reference));
    assert_ne!(first.scope.observation_socket_path(false).unwrap(), second.scope.observation_socket_path(false).unwrap());
    assert_ne!(first.head().stream.epoch.daemon.scope, second.head().stream.epoch.daemon.scope);
}

#[test]
fn reset_rotates_observation_epoch_and_rejects_already_queued_old_generation() {
    let fixture = Fixture::start(true);
    let before = fixture.head();
    let mut running = fixture.send_watermark("obs-reset-running");
    fixture.entered.recv_timeout(BOUND).unwrap();
    let mut reset = fixture.send("obs-reset", InvocationContext::default(), Request::Maintenance(crate::service::maintenance::Request::ResetConversation));
    fixture.wait_snapshot(|value| value.gauges.queued_jobs == 1);
    let mut stale = fixture.send_watermark("obs-stale-queued");
    fixture.wait_snapshot(|value| value.gauges.queued_jobs == 2);
    fixture.gate.release();
    assert!(matches!(super::frame::read(&mut running, Instant::now() + BOUND).unwrap(), ServerFrame::Result { result: Ok(_), .. }));
    let reset_generation = match super::frame::read(&mut reset, Instant::now() + BOUND).unwrap() {
        ServerFrame::Result { result: Ok(Response::Maintenance(crate::service::maintenance::Response::Reset(reset))), .. } => reset.database_generation,
        other => panic!("native reset failed: {}", serde_json::to_string(&other).unwrap()),
    };
    let stale_reply = super::frame::read(&mut stale, Instant::now() + BOUND).unwrap();
    assert!(matches!(stale_reply, ServerFrame::Failure(DaemonError::StaleGeneration) | ServerFrame::Result { result: Err(crate::service::ServiceError::StaleGeneration), .. }));
    fixture.wait_snapshot(|value| value.gauges.executing_jobs == 0 && value.gauges.queued_jobs == 0);
    let after = fixture.head();
    assert_eq!(before.stream.epoch.daemon, after.stream.epoch.daemon);
    assert_ne!(before.stream.epoch.database_generation, after.stream.epoch.database_generation);
    assert_eq!(after.stream.epoch.database_generation, reset_generation);
    let old_window = ObservationWindow { after: ObservationCursor { stream: before.stream.clone(), sequence: before.newest }, through: before.newest, filter: ObservationFilter::default() };
    match fixture.inspect(None, InspectionRequest::Events(ObservationRequest::Read(ObservationRead { window: old_window, limit: 1 }))).response {
        InspectionResponse::Events(Ok(ObservationResponse::Read(ObservationReadResult::EpochChanged { requested, head }))) => {
            assert_eq!(requested, before.stream.epoch);
            assert_eq!(head.stream, after.stream);
        }
        other => panic!("old reset window was not fenced: {other:?}"),
    }
    let stale = fixture.request("obs-stale-queued");
    assert!(matches!(stale.execution, ExecutionObservation::NotRun { reason: NotRunReason::Dispatch(RuntimeFailureCode::StaleGeneration), .. }));
}

#[test]
fn abandoned_mutation_reply_preserves_durable_acceptance_and_safe_observation() {
    use crate::agents::service::{Registration, Request as Participants, Response as ParticipantResponse};
    use crate::messaging::service::{Request as Messages, Response as MessageResponse, Submission};
    let fixture = Fixture::start(false);
    let before = fixture.head();
    let binding = uuid::Uuid::new_v4().to_string();
    let invocation = InvocationContext { process_binding_id: Some(binding.clone()), ..Default::default() };
    let registration = Registration { name: None, tool: "qoder".into(), session_id: None, process_binding_id: Some(binding), transcript_path: None, wait_timeout: 30, subagent_timeout: None, hints: None, auto_subscribe: String::new() };
    let participant = match fixture.call("obs-register", invocation, Request::Participants(Participants::Register(registration))).unwrap() {
        Response::Participants(ParticipantResponse::Registered(actor)) => actor.participant,
        other => panic!("fixture registration returned {other:?}"),
    };
    let text = "observation must not expose this arbitrary message body";
    *fixture.gate.held.lock().unwrap() = true;
    let stream = fixture.send(
        "obs-lost-reply",
        InvocationContext::default(),
        Request::Messages(Messages::Submit(Submission { recipients: vec![participant.canonical_name], text: text.into(), envelope: Default::default(), external_sender: Some("fixture-human".into()), broadcast_confirmed: false, inline_bundle: None })),
    );
    fixture.entered.recv_timeout(BOUND).unwrap();
    drop(stream);
    fixture.gate.release();
    fixture.wait_snapshot(|value| value.gauges.executing_jobs == 0 && value.gauges.queued_jobs == 0);
    let accepted = match fixture.call("obs-reconcile", InvocationContext::default(), Request::Messages(Messages::Acceptance { request_id: RequestId("obs-lost-reply".into()) })).unwrap() {
        Response::Messages(MessageResponse::Acceptance(Some(accepted))) => accepted,
        other => panic!("lost reply did not retain native acceptance: {other:?}"),
    };
    // Native Acceptance readback deliberately marks an existing outcome replayed.
    assert!(accepted.replayed, "reconciliation must return the stored acceptance");
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 3, "only registration, original Submit and explicit Acceptance lookup may enter Application");
    assert!(matches!(fixture.request("obs-lost-reply").execution, ExecutionObservation::Returned { .. }));
    let head = fixture.head();
    let loss_count = |head: &ObservationHead| {
        let value = &head.losses.dropped;
        value.contended + value.oversize + value.invalid_input + value.retired_epoch + value.collector_unavailable + value.preparation_budget
    };
    let losses_increased = loss_count(&head) > loss_count(&before);
    let window = ObservationWindow { after: ObservationCursor { stream: head.stream, sequence: 0 }, through: head.newest, filter: ObservationFilter { request_id: Some(RequestId("obs-lost-reply".into())), ..Default::default() } };
    let response = fixture.inspect(None, InspectionRequest::Events(ObservationRequest::Read(ObservationRead { window, limit: 256 }))).response;
    assert!(!serde_json::to_string(&response).unwrap().contains(text), "arbitrary business payload crossed the diagnostic boundary");
    match response {
        InspectionResponse::Events(Ok(ObservationResponse::Read(ObservationReadResult::Page(page)))) => {
            // Best-effort capture may be absent; the positive authoritative
            // acceptance above is never inferred from a missing event row.
            let mut projected = false;
            for record in page.records {
                if let ObservationPayload::DomainCommitted(crate::observability::events::DomainCommitObservation { fact: crate::observability::events::DomainCommitFact::MessageAccepted { acceptance, event, replayed }, .. }) = record.input.payload {
                    assert_eq!(acceptance.request_id.0, "obs-lost-reply");
                    assert_eq!(event.event, accepted.event);
                    assert!(!replayed, "the original committed projection must describe first acceptance");
                    projected = true;
                }
            }
            assert_eq!(fixture.calls.load(Ordering::SeqCst), 3, "passive reads must not replay or dispatch business work");
            assert!(projected || losses_increased, "postcommit acceptance projection was neither retained nor accounted as capture loss");
        }
        other => panic!("correlated diagnostic window failed: {other:?}"),
    }
}

/// Supplies real native observations to the independent browser driver. The
/// driver owns only adapter/browser processes; this guard owns the daemon.
#[test]
#[ignore = "requires released same-cohort browser product, bundle and driver"]
fn actual_browser_investigates_real_native_request_configuration_and_events() {
    use crate::agents::service::{Registration, Request as Participants, Response as ParticipantResponse};
    use crate::messaging::service::{InboxOperation, Request as Messages};
    use crate::observability::configuration::{ConfigObservationEvent, ConfigStage, InvocationCaptureHandle};
    use crate::observability::runtime::{ClientAttemptId, ClientInvocationId, OriginRef};
    let variable = |name| std::path::PathBuf::from(std::env::var_os(name).expect("select explicit consumer fixture inputs"));
    let binary = variable("AGENTX_ACCEPTANCE_BINARY").canonicalize().unwrap();
    let driver = variable("AGENTX_CONSUMER_DRIVER").canonicalize().unwrap();
    let node = variable("AGENTX_CONSUMER_NODE").canonicalize().unwrap();
    let assets = variable("AGENTX_CONSUMER_ASSETS").canonicalize().unwrap();
    let evidence = variable("AGENTX_OBSERVATION_EVIDENCE");
    std::fs::create_dir(&evidence).unwrap();
    let image = std::fs::read(&binary).unwrap();
    let stamp = super::wire::build_identity().as_bytes();
    assert!(image.windows(stamp.len()).any(|part| part == stamp), "browser product must share native fixture cohort");
    let fixture = Fixture::start(false);
    let binding = uuid::Uuid::new_v4().to_string();
    let registration = Registration { name: None, tool: "qoder".into(), session_id: None, process_binding_id: Some(binding.clone()), transcript_path: None, wait_timeout: 30, subagent_timeout: None, hints: None, auto_subscribe: String::new() };
    let participant = match fixture.call("browser-register", InvocationContext { process_binding_id: Some(binding), ..Default::default() }, Request::Participants(Participants::Register(registration))).unwrap() {
        Response::Participants(ParticipantResponse::Registered(actor)) => actor.participant,
        other => panic!("browser participant setup returned {other:?}"),
    };
    let mut captures = Vec::new();
    for (label, timeout) in [("left", "17"), ("right", "29")] {
        let origin = OriginRef { invocation: InvocationCaptureHandle::begin(ClientInvocationId::new()).reference(), attempt: ClientAttemptId::new() };
        let attempt = origin.attempt;
        let id = format!("browser-config-{label}");
        let invocation = InvocationContext {
            environment: std::collections::HashMap::from([("AGENTX_TIMEOUT".into(), timeout.into()), ("AGENTX_TERMINAL".into(), "private-browser-terminal -e {script}".into()), ("AGENTX_RELAY_TOKEN".into(), "private-browser-credential".into())]),
            observation_origin: Some(origin),
            ..Default::default()
        };
        assert!(matches!(fixture.call(&id, invocation, Request::Messages(Messages::Inbox(InboxOperation::Snapshot { participant: participant.clone() }))).unwrap(), Response::Messages(_)));
        let head = fixture.head();
        let window = ObservationWindow { after: ObservationCursor { stream: head.stream, sequence: 0 }, through: head.newest, filter: ObservationFilter { client_attempt: Some(attempt), ..Default::default() } };
        let response = fixture.inspect(None, InspectionRequest::Events(ObservationRequest::Read(ObservationRead { window, limit: 256 }))).response;
        let InspectionResponse::Events(Ok(ObservationResponse::Read(ObservationReadResult::Page(page)))) = response else { panic!("native browser config publication unavailable") };
        let mut refs = page.records.into_iter().filter_map(|record| match record.input.payload {
            ObservationPayload::Configuration(ConfigObservationEvent::SnapshotPublished { snapshot, attempt: Some(observed), stage: ConfigStage::ScopedEffective, .. }) if observed == attempt => Some(snapshot),
            _ => None,
        });
        let reference = refs.next().expect("real same-attempt configuration reference");
        assert!(refs.next().is_none());
        captures.push(serde_json::json!({"request_id":id,"attempt":attempt,"configuration":reference,"timeout":timeout}));
    }
    fixture.call("browser-request", InvocationContext::default(), Request::Events(crate::events::service::Request::Watermark)).unwrap();
    let request = fixture.request("browser-request");
    let snapshot = fixture.snapshot();
    let calls_before = fixture.calls.load(Ordering::SeqCst);
    let seed = evidence.join("native-seed.json");
    std::fs::write(&seed, serde_json::to_vec(&serde_json::json!({"scope":fixture.reference,"snapshot":snapshot,"request":request,"captures":captures,"head":fixture.head(),"business_calls":calls_before})).unwrap()).unwrap();
    let mut command = fixture.observer_command(&node);
    command.arg(&driver).arg("--binary").arg(&binary).arg("--assets").arg(&assets).arg("--seed").arg(&seed).arg("--evidence").arg(evidence.join("browser"));
    command.stdout(std::fs::File::create(evidence.join("driver.stdout")).unwrap()).stderr(std::fs::File::create(evidence.join("driver.stderr")).unwrap());
    let mut driver_child = ObserverChild(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(120);
    let status = loop {
        if let Some(status) = driver_child.0.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "consumer driver exceeded owned two-minute bound");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success(), "actual browser driver failed; retain exact driver logs");
    assert_eq!(fixture.calls.load(Ordering::SeqCst), calls_before, "browser/adapter must never dispatch business work");
    assert!(!fixture.thread.as_ref().unwrap().is_finished(), "browser cleanup stopped the native daemon");
    assert_eq!(fixture.head().stream.epoch.daemon, request.reference.daemon);
    println!("actual consumer driver passed; business calls unchanged={calls_before}; evidence={}", evidence.display());
}
