//! Composition cases use actual lease, kernel peer, owners and passive frames.
//! Authored only; native integration owns execution and fixture cleanup evidence.

use super::*;
use crate::observability::configuration::{ConfigurationQueryError, ConfigurationRequest};
use crate::observability::events::*;
use crate::service::{ServiceError, application::SqliteApplication};

struct ReleaseInitializer(Option<mpsc::SyncSender<()>>);
impl Drop for ReleaseInitializer {
    fn drop(&mut self) {
        if let Some(release) = self.0.take() {
            let _ = release.try_send(());
        }
    }
}

#[test]
fn initialization_failure_keeps_process_evidence_then_releases_observation_and_lease() {
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let reference = ScopeRef::from_canonical_scope(scope.root().to_path_buf()).unwrap();
    let (waiting, entered) = mpsc::sync_channel(1);
    let (release, released) = mpsc::sync_channel(1);
    let stop = Arc::new(AtomicBool::new(false));
    std::thread::scope(|threads| {
        let worker_scope = scope.clone();
        let worker = threads.spawn(move || {
            super::super::serve_with_stop(worker_scope, stop, move |_| {
                waiting.send(()).unwrap();
                released.recv_timeout(Duration::from_secs(10)).expect("release failing initializer");
                Err::<SqliteApplication, _>(ServiceError::Internal("fixture initialization failed".into()))
            })
        });
        let release = ReleaseInitializer(Some(release));
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        let result = inspect_existing(&reference, None, InspectionRequest::Runtime(RuntimeRequest::Snapshot)).unwrap();
        let InspectionResponse::Runtime(Ok(RuntimeResponse::Snapshot(snapshot))) = result.response else {
            panic!("expected process-only snapshot");
        };
        assert!(matches!(snapshot.lifecycle, DaemonLifecycle::Initializing));
        assert!(matches!(snapshot.generation.value, EvidenceValue::Unknown(UnknownReason::GenerationUnavailable)));
        assert!(matches!(
            inspect_existing(&reference, None, InspectionRequest::Configuration(ConfigurationRequest::Head { expected: None })).unwrap().response,
            InspectionResponse::Configuration(Err(ConfigurationError::Query(ConfigurationQueryError::GenerationUnavailable(_))))
        ));
        assert!(matches!(inspect_existing(&reference, None, InspectionRequest::Events(ObservationRequest::Describe)).unwrap().response, InspectionResponse::Events(Err(ObservationReadError::GenerationUnavailable { .. }))));
        drop(release);
        assert!(worker.join().unwrap().is_err());
    });
    assert!(!directory.path().join("agentx.db").exists());
    assert!(inspect_existing(&reference, None, InspectionRequest::Runtime(RuntimeRequest::Snapshot)).is_err());
    // A new lease succeeds only after all owned observation resources joined.
    drop(super::super::ScopeLease::acquire(scope).unwrap());
}

#[test]
fn reset_with_a_read_gate_never_redirects_a_retired_domain_fact() {
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let lease = super::super::ScopeLease::acquire(scope).unwrap();
    let state = ObservationState::new(&lease).unwrap();
    let first = uuid::Uuid::new_v4().to_string();
    lease.database_generation().publish(first.clone()).unwrap();
    state.synchronize_epoch();
    let head = state.journal.describe().unwrap();
    let sink = state.journal.sink().unwrap();
    let read_gate = state.publication.read().unwrap();
    let next = uuid::Uuid::new_v4().to_string();
    lease.database_generation().publish(next.clone()).unwrap();
    let connection = state.runtime.connection_guard(RuntimeConnectionKind::Inspection);
    let peer = AcceptedPeer { connection: connection.reference().unwrap().clone(), process: ObservedProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) }, observed_at: state.clock.now().unwrap() };
    assert!(matches!(
        state.route(&peer, InspectionCall { expected_daemon: state.daemon.clone(), request: InspectionRequest::Events(ObservationRequest::Describe) }, INSPECTION_RESPONSE_BYTES),
        InspectionResponse::Events(Err(ObservationReadError::Unavailable))
    ));
    assert!(matches!(
        sink.record(ObservationInput {
            epoch: head.stream.epoch.clone(),
            source: EvidenceStamp { origin: EvidenceOrigin::StoredDomainFact, at: state.clock.now().unwrap() },
            payload: ObservationPayload::DomainCommitted(DomainCommitObservation {
                correlation: DomainCorrelation { request: None, client_attempt: None, request_id: None, actor: None },
                fact: DomainCommitFact::EventAppended { event: DurableEventRef { scope: state.daemon.scope.clone(), database_generation: first, event: crate::messaging::inbox::EventId(1) }, kind: DomainEventKind::Status }
            })
        }),
        RecordDisposition::Dropped(DropReason::RetiredEpoch)
    ));
    drop(read_gate);
    state.synchronize_epoch();
    let current = state.journal.describe().unwrap();
    assert_eq!(current.stream.epoch.database_generation, next);
    assert_ne!(current.stream, head.stream);
    assert!(current.newest <= 1); // startup is best effort; no G1 domain input entered G2
}

#[test]
fn wrong_inspection_scope_and_unknown_operation_reject_before_owner_routing() {
    use std::os::unix::net::UnixStream;
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let lease = super::super::ScopeLease::acquire(scope).unwrap();
    let state = ObservationState::new(&lease).unwrap();
    for (scope, request, expected) in [
        (ScopeRef { canonical_scope: directory.path().join("wrong") }, serde_json::json!({"Runtime":"Snapshot"}), "scope"),
        (state.daemon.scope.clone(), serde_json::json!({"Runtime":"FutureInspection"}), "unsupported"),
        (state.daemon.scope.clone(), serde_json::json!({"Runtime":{"Connections":true}}), "input"),
    ] {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        client.set_nonblocking(true).unwrap();
        server.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + INSPECTION_TIMEOUT;
        frame::write_limited(&mut client, &InspectionClientFrame::Call { scope, expected_daemon: None, request }, deadline, INSPECTION_REQUEST_BYTES).unwrap();
        let error = connection(&mut server, &state, deadline).unwrap_err();
        assert!(match expected {
            "scope" => matches!(error, DaemonError::Incompatible(_)),
            "unsupported" => matches!(error, DaemonError::Unsupported(operation) if operation == "Runtime.FutureInspection"),
            _ => matches!(error, DaemonError::InvalidInput(_)),
        });
    }
    let connection = state.runtime.connection_guard(RuntimeConnectionKind::Inspection);
    let peer = AcceptedPeer { connection: connection.reference().unwrap().clone(), process: ObservedProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) }, observed_at: state.clock.now().unwrap() };
    let snapshot = state.runtime.snapshot(&state.daemon, &peer).unwrap();
    assert_eq!(snapshot.lifetime.accepted_connections, 0);
    assert_eq!(snapshot.lifetime.decoded_calls, 0);
    assert_eq!(snapshot.gauges.business_connections, 0);
}

#[test]
fn inspection_accepts_one_direct_call_and_returns_its_owner_identity() {
    use std::os::unix::net::UnixStream;
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let lease = super::super::ScopeLease::acquire(scope).unwrap();
    let state = ObservationState::new(&lease).unwrap();
    let (mut client, mut server) = UnixStream::pair().unwrap();
    client.set_nonblocking(true).unwrap();
    server.set_nonblocking(true).unwrap();
    std::thread::scope(|threads| {
        let deadline = Instant::now() + INSPECTION_TIMEOUT;
        let state_ref = &state;
        threads.spawn(move || connection(&mut server, state_ref, deadline).unwrap());
        frame::write_limited(&mut client, &InspectionClientFrame::Call { scope: state.daemon.scope.clone(), expected_daemon: None, request: InspectionRequest::Runtime(RuntimeRequest::Snapshot) }, deadline, INSPECTION_REQUEST_BYTES).unwrap();
        assert!(matches!(frame::read_limited::<InspectionServerFrame>(&mut client, deadline, INSPECTION_RESPONSE_BYTES).unwrap(),
            InspectionServerFrame::Result { ready, response: InspectionResponse::Runtime(Ok(RuntimeResponse::Snapshot(_))) } if ready.daemon == state.daemon));
    });
}

#[test]
fn inspection_client_accepts_response_with_different_diagnostic_protocol_and_retains_exact_instance() {
    let directory = tempfile::tempdir().unwrap();
    let scope = DaemonScope::prepare(directory.path()).unwrap();
    let lease = super::super::ScopeLease::acquire(scope).unwrap();
    let state = ObservationState::new(&lease).unwrap();
    let listener = platform::Listener::bind_observation(&lease).unwrap();
    std::thread::scope(|threads| {
        threads.spawn(|| {
            let deadline = Instant::now() + INSPECTION_TIMEOUT;
            assert!(listener.wait_ready(deadline).unwrap());
            let mut stream = listener.accept().unwrap().unwrap();
            let InspectionClientFrame::Call { scope, expected_daemon, request } = frame::read_limited::<InspectionClientFrame>(&mut stream, deadline, INSPECTION_REQUEST_BYTES).unwrap();
            assert_eq!(scope, state.daemon.scope);
            assert!(matches!(request, InspectionRequest::Runtime(RuntimeRequest::Snapshot)));
            let mut ready = state.ready().unwrap();
            ready.protocol_version += 1;
            ready.build_version = "another-version".into();
            ready.build_id = "another-build".into();
            let response = InspectionResponse::DaemonChanged { expected: expected_daemon.unwrap(), actual: state.daemon.clone() };
            frame::write_limited(&mut stream, &InspectionServerFrame::Result { ready, response }, deadline, INSPECTION_RESPONSE_BYTES).unwrap();
        });
        let mut old_daemon = state.daemon.clone();
        old_daemon.daemon_instance = "superseded".into();
        let result = inspect_existing(&state.daemon.scope, Some(old_daemon), InspectionRequest::Runtime(RuntimeRequest::Snapshot)).unwrap();
        assert_eq!(result.ready.build_id, "another-build");
        assert!(matches!(result.response, InspectionResponse::DaemonChanged { .. }));
    });
}

/// Held jobs represent genuine active workers, including post-Result tails.
fn held_inspection_workers() -> (InspectionWorkers, Vec<mpsc::SyncSender<()>>) {
    let mut workers = InspectionWorkers(Vec::new());
    let mut releases = Vec::new();
    for _ in 0..MAX_INSPECTION_WORKERS {
        let (release, released) = mpsc::sync_channel(1);
        let admitted = workers
            .try_spawn(move || {
                released.recv_timeout(Duration::from_secs(5)).unwrap();
            })
            .unwrap();
        assert!(admitted);
        releases.push(release);
    }
    (workers, releases)
}

fn finish_held_inspection_workers(workers: &InspectionWorkers, releases: Vec<mpsc::SyncSender<()>>) {
    for release in releases {
        release.send(()).unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while workers.0.iter().any(|worker| !worker.is_finished()) {
        assert!(Instant::now() < deadline, "released inspection worker did not finish");
        std::thread::yield_now();
    }
}

/// Model the exact accepted-socket race: both handles survive housekeeping,
/// then finish before the next admission. No scheduler luck or sleep is needed.
#[test]
fn inspection_admission_reaps_workers_finished_after_housekeeping() {
    let (mut workers, releases) = held_inspection_workers();
    workers.reap_finished();
    assert_eq!(workers.0.len(), MAX_INSPECTION_WORKERS);
    finish_held_inspection_workers(&workers, releases);
    assert_eq!(workers.0.len(), MAX_INSPECTION_WORKERS, "the old handles must still be retained at admission");
    let (ran, result) = mpsc::sync_channel(1);
    let admitted = workers
        .try_spawn(move || {
            ran.send(()).unwrap();
        })
        .unwrap();
    assert!(admitted, "finished handles cannot consume active capacity");
    result.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(workers.0.len(), 1);
}

#[test]
fn inspection_admission_rejects_live_capacity_then_recovers_and_joins() {
    let (mut workers, releases) = held_inspection_workers();
    let executed = Arc::new(AtomicBool::new(false));
    let third = executed.clone();
    let (mut client, accepted) = std::os::unix::net::UnixStream::pair().unwrap();
    client.set_read_timeout(Some(INSPECTION_TIMEOUT)).unwrap();
    let started = Instant::now();
    let admitted = workers
        .try_spawn(move || {
            drop(accepted);
            third.store(true, Ordering::SeqCst);
        })
        .unwrap();
    assert!(!admitted);
    let mut byte = [0];
    assert_eq!(std::io::Read::read(&mut client, &mut byte).unwrap(), 0, "saturated admission must close its accepted socket");
    assert!(started.elapsed() < INSPECTION_TIMEOUT, "capacity rejection must not wait for active workers");
    assert_eq!(workers.0.len(), MAX_INSPECTION_WORKERS);
    assert!(!executed.load(Ordering::SeqCst));
    finish_held_inspection_workers(&workers, releases);
    let completed = executed.clone();
    let admitted = workers
        .try_spawn(move || {
            completed.store(true, Ordering::SeqCst);
        })
        .unwrap();
    assert!(admitted);
    drop(workers);
    assert!(executed.load(Ordering::SeqCst), "drop must join admitted workers");
}
