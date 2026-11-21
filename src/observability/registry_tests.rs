//! Native state cases only. Real endpoint/32-worker isolation belongs to daemon integration.
use super::*;
use crate::observability::events::{DropReason, ObservationCursor};
use crate::service::{CallerProcess, RequestId};

fn fixture() -> (RuntimeRegistry, DaemonRef) {
    let scope = ScopeRef::from_canonical_scope(std::env::temp_dir().join("agentx-runtime-contract-fixture")).unwrap();
    let daemon = DaemonRef::from_lease(scope, "fixture-daemon".into()).unwrap();
    let registry = RuntimeRegistry::new(daemon.clone(), ObservationClock::new(), None).unwrap();
    let epoch = ObservationEpoch::from_committed_generation(daemon.clone(), uuid::Uuid::new_v4().to_string()).unwrap();
    registry.generation_published(epoch, registry.now().unwrap());
    registry.lifecycle(DaemonLifecycle::Serving, registry.now().unwrap());
    (registry, daemon)
}
fn connected(registry: &RuntimeRegistry, pid: u32, incarnation: Option<&str>) -> (RuntimeConnectionGuard, AcceptedPeer, PeerObservationRef) {
    let guard = registry.connection_guard(RuntimeConnectionKind::Business);
    let connection = guard.reference().unwrap().clone();
    let process = CallerProcess { pid, incarnation: incarnation.map(str::to_owned) };
    let at = registry.now().unwrap();
    let peer = registry.peer_observed(&connection, &process, at.clone()).into_reference().unwrap();
    (guard, AcceptedPeer { connection, process: ObservedProcess { pid, incarnation: process.incarnation }, observed_at: at }, peer)
}
fn decoded(registry: &RuntimeRegistry, accepted: &AcceptedPeer, peer: &PeerObservationRef, request_id: &str) -> RequestObservationRef {
    let at = registry.now().unwrap();
    let generation = registry.state.lock().unwrap().epoch().unwrap().database_generation;
    registry
        .call_decoded(
            DecodedCallInput {
                connection: Some(accepted.connection.clone()),
                peer: EvidenceValue::Present(peer.clone()),
                origin: None,
                request_id: &RequestId(request_id.into()),
                operation: OperationLabel { domain: OperationDomain::Messages, variant: "Submit".into() },
                caller_deadline_unix_ms: None,
                effective_deadline: EvidenceValue::Present(at.clone()),
                generation_at_decode: generation,
            },
            at,
        )
        .into_reference()
        .unwrap()
}
fn update(registry: &RuntimeRegistry, request: &RequestObservationRef, update: RequestUpdate) {
    registry.request_changed(Some(request), update, registry.now().unwrap());
}
fn detail(registry: &RuntimeRegistry, peer: &AcceptedPeer, request: &RequestObservationRef) -> RequestObservation {
    match registry.read(&request.daemon, peer, RuntimeRequest::RequestDetail(request.clone())).unwrap() {
        RuntimeResponse::RequestDetail(row) => row,
        _ => panic!("wrong result family"),
    }
}

#[test]
fn hello_only_connection_and_inspection_cleanup_never_make_a_business_call() {
    let (registry, daemon) = fixture();
    let (mut connection, accepted, _) = connected(&registry, 100, Some("A"));
    registry.connection_changed(connection.reference(), ConnectionUpdate::HelloValidated, registry.now().unwrap());
    connection.close_as(ConnectionCloseReason::HelloOnlyEof);
    drop(connection);
    let inspection = registry.connection_guard(RuntimeConnectionKind::Inspection);
    let snapshot = registry.snapshot(&daemon, &accepted).unwrap();
    assert_eq!(snapshot.gauges.business_connections, 0);
    assert_eq!(snapshot.gauges.inspection_connections, 1);
    assert_eq!(snapshot.lifetime.decoded_calls, 0);
    drop(inspection);
    assert_eq!(registry.snapshot(&daemon, &accepted).unwrap().gauges.inspection_connections, 0);
}

#[test]
fn busy_and_expired_queue_are_positive_not_run_evidence() {
    for reason in [RuntimeFailureCode::Busy, RuntimeFailureCode::Deadline] {
        let (registry, daemon) = fixture();
        let (_connection, accepted, peer) = connected(&registry, 100, Some("A"));
        let request = decoded(&registry, &accepted, &peer, "logical-operation");
        let worker = registry.worker_guard(Some(request.clone()));
        if reason == RuntimeFailureCode::Busy {
            let result: Result<(), ()> = registry.publish_queued(Some(&request), || Err(()), |_| RuntimeFailureCode::Busy);
            assert!(result.is_err());
        } else {
            let _: Result<(), ()> = registry.publish_queued(Some(&request), || Ok(()), |_| unreachable!());
            update(&registry, &request, RequestUpdate::ReplyWaitAbandoned(RuntimeFailureCode::Deadline));
            update(&registry, &request, RequestUpdate::Dequeued);
            update(&registry, &request, RequestUpdate::DispatchRejected(reason));
        }
        drop(worker);
        let row = detail(&registry, &accepted, &request);
        assert!(matches!(row.execution, ExecutionObservation::NotRun { .. }));
        let snapshot = registry.snapshot(&daemon, &accepted).unwrap();
        assert_eq!(snapshot.gauges.executing_jobs, 0);
        assert_eq!(snapshot.gauges.queued_jobs, 0);
        assert_eq!(snapshot.lifetime.dispatch_started, 0);
    }
}

#[test]
fn timeout_native_result_and_failed_write_remain_three_independent_facts() {
    let (registry, daemon) = fixture();
    let (_connection, accepted, peer) = connected(&registry, 100, Some("A"));
    let request = decoded(&registry, &accepted, &peer, "stable-business-id");
    let worker = registry.worker_guard(Some(request.clone()));
    let _: Result<(), ()> = registry.publish_queued(Some(&request), || Ok(()), |_| unreachable!());
    update(&registry, &request, RequestUpdate::Dequeued);
    let generation = registry.state.lock().unwrap().epoch().unwrap().database_generation;
    update(&registry, &request, RequestUpdate::DispatchStarted { generation: generation.clone() });
    update(&registry, &request, RequestUpdate::ReplyWaitAbandoned(RuntimeFailureCode::Deadline));
    update(&registry, &request, RequestUpdate::ReplyWriteFinished { failure: Some(RuntimeFailureCode::Io) });
    drop(worker);
    assert_eq!(registry.snapshot(&daemon, &accepted).unwrap().gauges.executing_jobs, 1);
    assert!(registry.state.lock().unwrap().active.contains_key(&request.id.0));
    update(&registry, &request, RequestUpdate::ApplicationReturned { result: ApplicationResultClass::ReturnedOk, generation: EvidenceValue::Present(generation), post_commit_work: None });
    let row = detail(&registry, &accepted, &request);
    assert!(matches!(row.execution, ExecutionObservation::Returned { result: ApplicationResultClass::ReturnedOk, .. }));
    assert!(matches!(row.reply, ReplyObservation::Observed { wait: ReplyWaitObservation::Abandoned { .. }, write: ReplyWriteObservation::Failed { .. } }));
    assert!(row.enqueued_at.is_some() && row.dequeued_at.is_some());
    assert!(!registry.state.lock().unwrap().active.contains_key(&request.id.0));
}

#[test]
fn ordinary_long_scope_request_retains_full_lifecycle_and_configuration() {
    use crate::observability::configuration::{ConfigCaptureResult, ConfigSnapshotRef, ConfigStage, InvocationCaptureRef};

    // Same scope shape as the real detached LAN demonstration, without touching disk.
    let scope = ScopeRef::from_canonical_scope(std::path::PathBuf::from("/home/long/.local/state/agentx-observability-demo/data")).unwrap();
    let daemon = DaemonRef::from_lease(scope, uuid::Uuid::new_v4().to_string()).unwrap();
    let registry = RuntimeRegistry::new(daemon.clone(), ObservationClock::new(), None).unwrap();
    let epoch = ObservationEpoch::from_committed_generation(daemon.clone(), uuid::Uuid::new_v4().to_string()).unwrap();
    registry.generation_published(epoch.clone(), registry.now().unwrap());
    registry.lifecycle(DaemonLifecycle::Serving, registry.now().unwrap());
    let (_connection, accepted, peer) = connected(&registry, 41237, Some("90823741"));
    let origin = OriginRef { invocation: InvocationCaptureRef { invocation: ClientInvocationId::new(), revision: 1 }, attempt: ClientAttemptId::new() };
    let request_id = RequestId(uuid::Uuid::new_v4().to_string());
    let at = registry.now().unwrap();
    let request = registry
        .call_decoded(
            DecodedCallInput {
                connection: Some(accepted.connection.clone()),
                peer: EvidenceValue::Present(peer.clone()),
                origin: Some(origin.clone()),
                request_id: &request_id,
                operation: OperationLabel { domain: OperationDomain::Messages, variant: "Submit".into() },
                caller_deadline_unix_ms: Some(1_794_095_100_000),
                effective_deadline: EvidenceValue::Present(at.clone()),
                generation_at_decode: epoch.database_generation.clone(),
            },
            at,
        )
        .into_reference()
        .unwrap();
    assert_eq!(detail(&registry, &accepted, &request).origin, Some(origin.clone()));
    let worker = registry.worker_guard(Some(request.clone()));
    let _: Result<(), ()> = registry.publish_queued(Some(&request), || Ok(()), |_| unreachable!());
    update(&registry, &request, RequestUpdate::Dequeued);
    update(&registry, &request, RequestUpdate::DispatchStarted { generation: epoch.database_generation.clone() });
    // Exercise the full supported trace, preserving every supplied reference.
    for sequence in 1..=8 {
        update(&registry, &request, RequestUpdate::ConfigurationUsed(ConfigurationUse { stage: ConfigStage::ScopedEffective, capture: ConfigCaptureResult::Recorded(ConfigSnapshotRef { epoch: epoch.clone(), sequence }) }));
    }
    update(&registry, &request, RequestUpdate::ApplicationReturned { result: ApplicationResultClass::ReturnedOk, generation: EvidenceValue::Present(epoch.database_generation.clone()), post_commit_work: None });
    update(&registry, &request, RequestUpdate::ReplyResponseAvailable);
    update(&registry, &request, RequestUpdate::ReplyWriteFinished { failure: None });
    drop(worker);

    let row = detail(&registry, &accepted, &request);
    assert_eq!(row.reference, request);
    assert_eq!(row.connection, accepted.connection);
    assert!(matches!(&row.peer, EvidenceValue::Present(value) if value == &peer));
    assert_eq!(row.origin, Some(origin));
    assert_eq!(row.request_id, request_id);
    assert!(row.enqueued_at.is_some() && row.dequeued_at.is_some());
    assert!(matches!(row.effective_deadline, EvidenceValue::Present(_)));
    assert_eq!(row.caller_deadline_unix_ms, Some(1_794_095_100_000));
    assert_eq!(row.generation_at_decode, epoch.database_generation);
    assert_eq!(row.generation_at_dispatch.as_ref(), Some(&epoch.database_generation));
    assert!(matches!(&row.generation_after_return, EvidenceValue::Present(value) if value == &epoch.database_generation));
    assert!(matches!(row.execution, ExecutionObservation::Returned { result: ApplicationResultClass::ReturnedOk, .. }));
    assert!(matches!(row.reply, ReplyObservation::Observed { wait: ReplyWaitObservation::ResponseAvailable { .. }, write: ReplyWriteObservation::Finished { .. } }));
    let ConfigurationTrace::Captured { uses, omitted_uses } = &row.configuration else { panic!("configuration trace lost") };
    assert_eq!(*omitted_uses, 0);
    assert_eq!(uses.len(), 8);
    for (index, usage) in uses.iter().enumerate() {
        assert_eq!(usage.stage, ConfigStage::ScopedEffective);
        assert!(matches!(&usage.capture, ConfigCaptureResult::Recorded(reference) if reference.epoch == epoch && reference.sequence == index as u64 + 1));
    }
    // This ordinary row exceeds the former admission budget without padded labels.
    assert!(serde_json::to_vec(&row).unwrap().len() > 2048);
    let RuntimeResponse::Requests(page) = registry.read(&daemon, &accepted, RuntimeRequest::Requests { select: RequestSelection::AllRetained, page: RuntimeList { before: None, limit: 64 } }).unwrap() else { panic!("wrong result family") };
    assert_eq!(page.rows.len(), 1);
    assert_eq!(page.rows[0].reference, request);
    let snapshot = registry.snapshot(&daemon, &accepted).unwrap();
    assert_eq!(snapshot.coverage.request_details_dropped, 0);
    assert_eq!(snapshot.gauges.executing_jobs, 0);
    assert_eq!(snapshot.lifetime.decoded_calls, 1);
    assert_eq!(snapshot.lifetime.application_ok, 1);
    assert_eq!(snapshot.lifetime.reply_write_ok, 1);
    assert!(!registry.state.lock().unwrap().active.contains_key(&request.id.0));
}

#[test]
fn kernel_incarnation_not_pid_or_unknown_equality_groups_peers() {
    let (registry, _) = fixture();
    let (_a, _, a) = connected(&registry, 100, Some("A"));
    let (_b, _, b) = connected(&registry, 100, Some("B"));
    let (_u1, _, u1) = connected(&registry, 101, None);
    let (_u2, _, u2) = connected(&registry, 101, None);
    assert_ne!(a, b);
    assert_ne!(u1, u2);
    let (_same, _, same) = connected(&registry, 100, Some("A"));
    assert_eq!(a, same);
}

#[test]
fn queue_gate_prevents_dispatch_before_queue_publication() {
    let (registry, _) = fixture();
    let (_connection, accepted, peer) = connected(&registry, 100, Some("A"));
    let request = decoded(&registry, &accepted, &peer, "queue-race");
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let dispatcher = registry.clone();
    let generation = registry.state.lock().unwrap().epoch().unwrap().database_generation;
    let joined = std::thread::spawn(move || {
        let request = receiver.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        update(&dispatcher, &request, RequestUpdate::Dequeued);
        update(&dispatcher, &request, RequestUpdate::DispatchStarted { generation });
    });
    registry.publish_queued(Some(&request), || sender.try_send(request.clone()), |_| RuntimeFailureCode::Busy).unwrap();
    joined.join().unwrap();
    assert!(matches!(detail(&registry, &accepted, &request).execution, ExecutionObservation::Running { .. }));
}

#[test]
fn reset_preserves_request_era_and_lifetime_but_clears_recent_metrics() {
    let (registry, daemon) = fixture();
    let (_connection, accepted, peer) = connected(&registry, 100, Some("A"));
    let request = decoded(&registry, &accepted, &peer, "old-generation");
    let old_generation = registry.state.lock().unwrap().epoch().unwrap().database_generation;
    let _: Result<(), ()> = registry.publish_queued(Some(&request), || Ok(()), |_| unreachable!());
    let new_epoch = ObservationEpoch::from_committed_generation(daemon.clone(), uuid::Uuid::new_v4().to_string()).unwrap();
    registry.generation_published(new_epoch.clone(), registry.now().unwrap());
    update(&registry, &request, RequestUpdate::Dequeued);
    update(&registry, &request, RequestUpdate::DispatchRejected(RuntimeFailureCode::StaleGeneration));
    update(&registry, &request, RequestUpdate::WorkerFinished);
    let row = detail(&registry, &accepted, &request);
    assert_eq!(row.generation_at_decode, old_generation);
    let snapshot = registry.snapshot(&daemon, &accepted).unwrap();
    assert_eq!(snapshot.recent.epoch.unwrap(), new_epoch);
    assert_eq!(snapshot.recent.counters.dispatch_not_run, 0);
    assert_eq!(snapshot.lifetime.dispatch_not_run, 1);
    let mut replacement = daemon.clone();
    replacement.daemon_instance = "replacement".into();
    assert!(matches!(registry.read(&replacement, &accepted, RuntimeRequest::Snapshot), Err(RuntimeReadError::DaemonChanged { .. })));
}

#[test]
fn oversized_detail_does_not_erase_finite_active_gauges() {
    let (registry, daemon) = fixture();
    let (_connection, accepted, peer) = connected(&registry, 100, Some("A"));
    let request = decoded(&registry, &accepted, &peer, &"x".repeat(1025));
    let _: Result<(), ()> = registry.publish_queued(Some(&request), || Ok(()), |_| unreachable!());
    let snapshot = registry.snapshot(&daemon, &accepted).unwrap();
    assert_eq!(snapshot.gauges.queued_jobs, 1);
    assert_eq!(snapshot.coverage.request_details_dropped, 1);
    assert!(matches!(registry.read(&daemon, &accepted, RuntimeRequest::RequestDetail(request)), Err(RuntimeReadError::DetailUnavailable(_))));
}

fn report(registry: &RuntimeRegistry) -> ClientAttemptReport {
    let at = registry.now().unwrap();
    ClientAttemptReport {
        attempt: ClientAttemptId::new(),
        kind: ClientAttemptKind::Probe,
        invocation: None,
        request_id: None,
        requested_scope: std::path::PathBuf::from("absent-relative-scope"),
        resolved_scope: EvidenceValue::Unknown(UnknownReason::ObservationFailed),
        started_at: at.clone(),
        ended_at: at.clone(),
        stages: vec![],
        omitted_stages: 0,
        last_ready: None,
        original_epoch: None,
        knowledge: ClientCallKnowledge::DiscoveryOnly,
        failure: Some(LocalFailure { stage: ClientStage::Connect, kind: LocalFailureKind::ConnectionRefused, os_code: None, at: Some(at) }),
    }
}
#[test]
fn report_intake_preserves_submitter_and_cannot_overwrite_another_peer() {
    let (registry, daemon) = fixture();
    let (_a, accepted_a, _) = connected(&registry, 100, Some("A"));
    let (_b, accepted_b, _) = connected(&registry, 100, Some("B"));
    let original = report(&registry);
    let a = registry.report_client(&daemon, &accepted_a, original.clone()).unwrap();
    let b = registry.report_client(&daemon, &accepted_b, original.clone()).unwrap();
    assert_ne!(a.reference, b.reference);
    let mut conflicting = original;
    conflicting.failure = None;
    assert!(matches!(registry.report_client(&daemon, &accepted_a, conflicting), Err(RuntimeReadError::InvalidInput)));
    if let RuntimeResponse::ClientReportDetail(row) = registry.read(&daemon, &accepted_a, RuntimeRequest::ClientReportDetail(b.reference.unwrap())).unwrap() {
        assert_eq!(row.observation.submitted_by.process.incarnation.as_deref(), Some("B"));
    } else {
        panic!("wrong report response");
    }
}

#[test]
fn completed_eviction_and_unwind_release_are_bounded_without_replaying_work() {
    let (registry, daemon) = fixture();
    let (_connection, accepted, peer) = connected(&registry, 100, Some("A"));
    let request = decoded(&registry, &accepted, &peer, "unwind-case");
    let worker = registry.worker_guard(Some(request.clone()));
    let dispatcher = registry.dispatch_guard(Some(request.clone()));
    let generation = registry.state.lock().unwrap().epoch().unwrap().database_generation;
    update(&registry, &request, RequestUpdate::DispatchStarted { generation });
    drop(worker);
    drop(dispatcher);
    assert_eq!(registry.snapshot(&daemon, &accepted).unwrap().gauges.executing_jobs, 0);
    assert!(matches!(detail(&registry, &accepted, &request).execution, ExecutionObservation::Interrupted { .. }));
    let mut later = registry.now().unwrap();
    later.monotonic_ms.0 += HISTORY_AGE_MS;
    registry.maintain(later);
    assert!(matches!(registry.read(&daemon, &accepted, RuntimeRequest::RequestDetail(request)), Err(RuntimeReadError::DetailUnavailable(_))));
}

struct DroppingSink;
impl ObservationSink for DroppingSink {
    fn record(&self, _: ObservationInput) -> RecordDisposition {
        RecordDisposition::Dropped(DropReason::Contended)
    }
    fn note_drop(&self, _: DropReason, _: u64) {}
}
#[test]
fn journal_loss_and_reader_contention_do_not_change_application_facts() {
    let (registry, daemon) = fixture();
    registry.set_sink(Some(Arc::new(DroppingSink)));
    let (_connection, accepted, peer) = connected(&registry, 100, Some("A"));
    let request = decoded(&registry, &accepted, &peer, "drop-event");
    let generation = registry.state.lock().unwrap().epoch().unwrap().database_generation;
    update(&registry, &request, RequestUpdate::DispatchStarted { generation });
    let held = registry.state.lock().unwrap();
    assert!(matches!(registry.read(&daemon, &accepted, RuntimeRequest::Snapshot), Err(RuntimeReadError::Busy)));
    drop(held);
    assert!(matches!(detail(&registry, &accepted, &request).execution, ExecutionObservation::Running { .. }));
    let mut event = RuntimeEvent {
        at: registry.now().unwrap(),
        source: RuntimeProducer::Dispatcher,
        correlation: RuntimeCorrelation { daemon: daemon.clone(), epoch: registry.state.lock().unwrap().epoch(), connection: None, request: Some(request.id), client_attempt: Some(ClientAttemptId::new()), request_id: None },
        fact: RuntimeFact::DispatchStarted,
    };
    assert!(event.validate());
    event.correlation.client_attempt = Some(ClientAttemptId(uuid::Uuid::nil()));
    assert!(!event.validate());
    let _: Option<ObservationCursor> = None;
}

#[test]
fn connection_limits_do_not_allocate_an_extra_worker_and_drop_releases_both_lanes() {
    let (registry, daemon) = fixture();
    let (_caller, accepted, _) = connected(&registry, 100, Some("A"));
    let mut business = Vec::new();
    for _ in 0..31 {
        business.push(registry.connection_guard(RuntimeConnectionKind::Business));
    }
    let overflow = registry.connection_guard(RuntimeConnectionKind::Business);
    assert!(overflow.reference().is_none());
    let inspection = [registry.connection_guard(RuntimeConnectionKind::Inspection), registry.connection_guard(RuntimeConnectionKind::Inspection)];
    let extra = registry.connection_guard(RuntimeConnectionKind::Inspection);
    assert!(extra.reference().is_none());
    let snapshot = registry.snapshot(&daemon, &accepted).unwrap();
    assert_eq!(snapshot.gauges.business_connections, 32);
    assert_eq!(snapshot.gauges.inspection_connections, 2);
    drop(business);
    drop(inspection);
    let snapshot = registry.snapshot(&daemon, &accepted).unwrap();
    assert_eq!(snapshot.gauges.business_connections, 1);
    assert_eq!(snapshot.gauges.inspection_connections, 0);
}

#[test]
fn originating_failure_reports_are_local_bounded_and_never_replay_a_call() {
    let (registry, _) = fixture();
    let mut reports = LocalAttemptReports::default();
    for _ in 0..40 {
        assert!(reports.push(report(&registry)));
    }
    assert_eq!(reports.len(), 32);
    assert_eq!(reports.dropped(), 8);
    let mut invalid = report(&registry);
    invalid.ended_at.clock = ObservationClock::new().id().clone();
    assert!(!reports.push(invalid));
    let first = reports.pop_oldest().unwrap();
    assert!(matches!(first.failure, Some(LocalFailure { stage: ClientStage::Connect, .. })));
    assert!(matches!(first.knowledge, ClientCallKnowledge::DiscoveryOnly));
    assert!(first.original_epoch.is_none());
}

#[test]
fn missing_local_failure_time_preserves_cause_without_relaxing_report_clocks() {
    let (registry, _) = fixture();
    let mut terminal = report(&registry);
    terminal.failure = Some(LocalFailure { stage: ClientStage::Connect, kind: LocalFailureKind::ConnectionRefused, os_code: Some(111), at: None });
    assert!(terminal.validate());
    assert!(matches!(terminal.failure, Some(LocalFailure { stage: ClientStage::Connect, kind: LocalFailureKind::ConnectionRefused, os_code: Some(111), at: None })));

    terminal.failure.as_mut().unwrap().at = Some(terminal.ended_at.clone());
    assert!(terminal.validate());
    let mut outside = terminal.ended_at.clone();
    outside.monotonic_ms.0 += 1;
    terminal.failure.as_mut().unwrap().at = Some(outside);
    assert!(!terminal.validate());
    let mut foreign = terminal.ended_at.clone();
    foreign.clock = ObservationClock::new().id().clone();
    terminal.failure.as_mut().unwrap().at = Some(foreign);
    assert!(!terminal.validate());

    terminal.failure = Some(LocalFailure { stage: ClientStage::Capture, kind: LocalFailureKind::ClockUnavailable, os_code: None, at: None });
    assert!(terminal.validate());
    terminal.failure.as_mut().unwrap().os_code = Some(111);
    assert!(!terminal.validate());
    terminal.failure.as_mut().unwrap().os_code = None;
    terminal.failure.as_mut().unwrap().at = Some(terminal.ended_at.clone());
    assert!(!terminal.validate());
    terminal.failure.as_mut().unwrap().at = None;
    terminal.ended_at.clock = ObservationClock::new().id().clone();
    assert!(!terminal.validate());
}
