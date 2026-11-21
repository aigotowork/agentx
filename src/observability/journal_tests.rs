//! Owner-authored semantic cases. Execution belongs to independent integration.

use super::*;
use crate::messaging::inbox::{BatchId, ClaimPhase, EventId};
use crate::observability::context::{EvidenceOrigin, EvidenceStamp, MonotonicMillis, ScopeRef};
use crate::observability::runtime::{ClientAttemptId, RequestObservationId};
use crate::service::RequestId;

struct Fixture {
    journal: ObservationJournal,
    epoch: ObservationEpoch,
    clock: ObservationClock,
}

impl Fixture {
    fn new() -> Self {
        let scope = ScopeRef { canonical_scope: std::env::temp_dir().join("agentx-journal-value-only") };
        Self::for_scope(scope)
    }

    fn for_scope(scope: ScopeRef) -> Self {
        let daemon = DaemonRef { scope, daemon_instance: "journal-daemon".into() };
        let clock = ObservationClock::new();
        let epoch = ObservationEpoch { daemon: daemon.clone(), database_generation: uuid::Uuid::new_v4().to_string() };
        let journal = ObservationJournal::new(daemon, clock.clone()).unwrap();
        journal.publish_epoch(epoch.clone()).unwrap();
        Self { journal, epoch, clock }
    }

    fn input(&self, event: i64) -> ObservationInput {
        ObservationInput {
            epoch: self.epoch.clone(),
            source: EvidenceStamp { origin: EvidenceOrigin::StoredDomainFact, at: self.clock.now().unwrap() },
            payload: ObservationPayload::DomainCommitted(DomainCommitObservation {
                correlation: DomainCorrelation::default(),
                fact: DomainCommitFact::EventAppended { event: DurableEventRef { scope: self.epoch.daemon.scope.clone(), database_generation: self.epoch.database_generation.clone(), event: EventId(event) }, kind: DomainEventKind::Message },
            }),
        }
    }

    fn append(&self, event: i64) -> ObservationCursor {
        match self.journal.sink().unwrap().record(self.input(event)) {
            RecordDisposition::Recorded(cursor) => cursor,
            other => panic!("unexpected capture disposition: {other:?}"),
        }
    }

    fn window(&self, after: u64, through: u64) -> ObservationWindow {
        ObservationWindow { after: ObservationCursor { stream: self.journal.describe().unwrap().stream, sequence: after }, through, filter: ObservationFilter::default() }
    }

    fn read(&self, window: ObservationWindow, limit: u16) -> ObservationReadResult {
        self.journal.read(ObservationRead { window, limit }).unwrap()
    }
}

fn page(value: ObservationReadResult) -> ObservationPage {
    match value {
        ObservationReadResult::Page(page) => page,
        other => panic!("expected page, got {other:?}"),
    }
}

fn sequences(page: &ObservationPage) -> Vec<u64> {
    page.records.iter().map(|r| r.cursor.sequence).collect()
}

#[test]
fn filtered_progress_scans_nonmatches_and_limits_candidate_work() {
    let f = Fixture::new();
    for i in 1..=1030 {
        f.append(i);
    }
    let mut window = f.window(0, 1030);
    window.filter.family = Some(ObservationFamily::Runtime);
    let first = page(f.read(window, 20));
    assert!(first.records.is_empty());
    assert_eq!(first.next.after.sequence, MAX_SCAN_RECORDS as u64);
    let second = page(f.read(first.next, 20));
    assert!(second.records.is_empty());
    assert_eq!(second.next.after.sequence, 1030);

    let mut selected = f.window(0, 3);
    selected.filter.domain_event = Some(DurableEventRef { scope: f.epoch.daemon.scope.clone(), database_generation: f.epoch.database_generation.clone(), event: EventId(1) });
    let result = page(f.read(selected, 20));
    assert_eq!(sequences(&result), [1]);
    assert_eq!(result.next.after.sequence, 3); // Last matching sequence was 1.
}

#[test]
fn frozen_frontier_survives_append_and_read_replay() {
    let f = Fixture::new();
    f.append(1);
    f.append(2);
    let first = page(f.read(f.window(0, 2), 1));
    assert_eq!(sequences(&first), [1]);
    assert_eq!(first.next.through, 2);
    let repeat = page(f.read(f.window(0, 2), 1));
    assert_eq!(repeat.records[0].cursor, first.records[0].cursor);
    f.append(3);
    let second = page(f.read(first.next, 1));
    assert_eq!(sequences(&second), [2]);
    assert_eq!(second.next.after.sequence, 2);
    let tail = page(f.read(f.window(2, 3), 1));
    assert_eq!(sequences(&tail), [3]);
}

#[test]
fn byte_blocked_match_is_not_scanned_or_skipped() {
    let f = Fixture::new();
    f.append(1);
    f.append(2);
    f.append(3);
    let input = ObservationRead { window: f.window(0, 3), limit: 3 };
    let mut empty = ObservationPage { records: Vec::new(), next: input.window.clone(), head: f.journal.describe().unwrap() };
    empty.next.after.sequence = empty.next.through;
    let envelope = encode_bounded(&ObservationResponse::Read(ObservationReadResult::Page(empty)), MAX_PAGE_BYTES).unwrap().len() + 128;
    let record_size = f.journal.inner.state.lock().unwrap().as_ref().unwrap().records[0].encoded.len();
    let first = page(f.journal.read_at(input, Some(f.clock.now().unwrap()), envelope + record_size + 4).unwrap());
    assert_eq!(sequences(&first), [1]);
    assert_eq!(first.next.after.sequence, 1);
    let rest = page(f.read(first.next, 10));
    assert_eq!(sequences(&rest), [2, 3]);
}

#[test]
fn prefix_eviction_is_a_gap_but_complete_expired_window_is_complete() {
    let f = Fixture::new();
    f.append(1);
    f.append(2);
    let old = f.window(0, 2);
    let completed = f.window(2, 2);
    let mut now = f.clock.now().unwrap();
    now.monotonic_ms = MonotonicMillis(now.monotonic_ms.0 + MAX_AGE_MS);
    let result = f.journal.read_at(ObservationRead { window: old, limit: 10 }, Some(now.clone()), MAX_PAGE_BYTES).unwrap();
    assert!(matches!(result, ObservationReadResult::Gap { first_missing: 1, last_missing: 2, .. }));
    let complete = page(f.journal.read_at(ObservationRead { window: completed, limit: 10 }, Some(now), MAX_PAGE_BYTES).unwrap());
    assert!(complete.records.is_empty());
    assert_eq!(complete.next.after.sequence, 2);
    assert_eq!(complete.head.oldest, 3);
    assert_eq!(complete.head.losses.evicted.age, 2);
}

#[test]
fn count_pressure_between_pages_reports_exact_missing_prefix() {
    let f = Fixture::new();
    f.append(1);
    f.append(2);
    f.append(3);
    let first = page(f.read(f.window(0, 3), 1));
    for i in 4..=(MAX_RECORDS as i64 + 2) {
        f.append(i);
    }
    let result = f.read(first.next, 10);
    match result {
        ObservationReadResult::Gap { first_missing, last_missing, head } => {
            assert_eq!((first_missing, last_missing), (2, 2));
            assert_eq!(head.losses.evicted.count_pressure, 2);
            assert_eq!(head.oldest, 3);
        }
        other => panic!("expected gap, got {other:?}"),
    }
}

#[test]
fn future_inverted_and_invalid_page_inputs_fail_without_clamping() {
    let f = Fixture::new();
    f.append(1);
    for (after, through, limit) in [(0, 2, 1), (1, 0, 1), (0, 1, 0), (0, 1, 257)] {
        assert!(matches!(f.journal.read(ObservationRead { window: f.window(after, through), limit }), Err(ObservationReadError::InvalidInput(_))));
    }
    let mut window = f.window(0, 1);
    window.after.stream.epoch.daemon.scope.canonical_scope.push("different-scope");
    assert!(matches!(f.journal.read(ObservationRead { window, limit: 1 }), Err(ObservationReadError::InvalidInput(ReadInputError::WrongScope))));
}

#[test]
fn collector_replacement_and_reset_fence_old_handles_and_cursors() {
    let f = Fixture::new();
    let old_sink = f.journal.sink().unwrap();
    f.append(1);
    let old_window = f.window(1, 1);
    let replacement = f.journal.replace_stream().unwrap();
    assert!(matches!(f.read(old_window.clone(), 1), ObservationReadResult::StreamChanged { .. }));
    assert!(matches!(old_sink.record(f.input(2)), RecordDisposition::Dropped(DropReason::RetiredEpoch)));
    let new = f.journal.sink().unwrap();
    let mut next_epoch = f.epoch.clone();
    next_epoch.database_generation = uuid::Uuid::new_v4().to_string();
    f.journal.unpublish_epoch().unwrap();
    assert!(matches!(f.journal.describe(), Err(ObservationReadError::GenerationUnavailable { .. })));
    f.journal.publish_epoch(next_epoch).unwrap();
    assert!(matches!(f.read(old_window, 1), ObservationReadResult::EpochChanged { .. }));
    assert!(matches!(new.record(f.input(2)), RecordDisposition::Dropped(DropReason::RetiredEpoch)));
    assert_ne!(replacement, f.journal.describe().unwrap().stream);
    assert_eq!(f.journal.describe().unwrap().newest, 0);
}

#[test]
fn restarted_daemon_is_an_epoch_change_before_numeric_boundary_validation() {
    let f = Fixture::new();
    f.append(1);
    let old = f.window(1, 1);
    let mut successor = f.epoch.daemon.clone();
    successor.daemon_instance = "new-daemon".into();
    let journal = ObservationJournal::new(successor.clone(), ObservationClock::new()).unwrap();
    journal.publish_epoch(ObservationEpoch { daemon: successor, database_generation: f.epoch.database_generation }).unwrap();
    assert!(matches!(journal.read(ObservationRead { window: old, limit: 1 }), Ok(ObservationReadResult::EpochChanged { .. })));
}

#[test]
fn capture_drops_do_not_allocate_sequences_or_forge_retention_gaps() {
    let f = Fixture::new();
    let sink = f.journal.sink().unwrap();
    let held = f.journal.inner.state.lock().unwrap();
    assert!(matches!(sink.record(f.input(1)), RecordDisposition::Dropped(DropReason::Contended)));
    assert!(matches!(f.journal.describe(), Err(ObservationReadError::Busy)));
    drop(held);
    let mut invalid = f.input(1);
    invalid.source.origin = EvidenceOrigin::ClientReported;
    assert!(matches!(sink.record(invalid), RecordDisposition::Dropped(DropReason::InvalidInput)));
    sink.note_drop(DropReason::PreparationBudget, 3);
    let recorded = f.append(2);
    assert_eq!(recorded.sequence, 1);
    let result = page(f.read(f.window(0, 1), 10));
    assert_eq!(sequences(&result), [1]);
    assert_eq!(result.head.losses.dropped.contended, 1);
    assert_eq!(result.head.losses.dropped.invalid_input, 1);
    assert_eq!(result.head.losses.dropped.preparation_budget, 3);
    assert_eq!(result.head.losses.evicted.count_pressure, 0);
}

#[test]
fn closed_or_dropped_owner_does_not_live_through_producer_handles() {
    let f = Fixture::new();
    let sink = f.journal.sink().unwrap();
    let input = f.input(1);
    f.journal.close();
    assert!(matches!(sink.record(input.clone()), RecordDisposition::Dropped(DropReason::CollectorUnavailable)));
    drop(f);
    assert!(matches!(sink.record(input), RecordDisposition::Dropped(DropReason::CollectorUnavailable)));
}

#[test]
fn safe_output_projection_cannot_reconstitute_native_receipt() {
    let f = Fixture::new();
    let mut input = f.input(1);
    input.payload = ObservationPayload::DomainCommitted(DomainCommitObservation {
        correlation: DomainCorrelation::default(),
        fact: DomainCommitFact::OutputConfirmationStored {
            output: OutputEvidenceRef {
                scope: f.epoch.daemon.scope.clone(),
                database_generation: f.epoch.database_generation.clone(),
                participant: crate::agents::attachments::ParticipantRef { canonical_name: "alice".into(), incarnation_bits: 1 },
                observation: OutputObservationId::new(),
            },
            through: EventId(9),
        },
    });
    let bytes = encode_bounded(&input, MAX_RECORD_BYTES).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let output = &json["payload"]["DomainCommitted"]["fact"]["OutputConfirmationStored"]["output"];
    assert!(output.get("observation").is_some());
    assert!(output.get("id").is_none());
    assert!(output.get("output_id").is_none());
    assert!(input.validate());
}

#[test]
fn server_request_filter_is_collection_daemon_qualified() {
    let f = Fixture::new();
    let mut input = f.input(1);
    let ObservationPayload::DomainCommitted(value) = &mut input.payload else { unreachable!() };
    value.correlation.request = Some(RequestObservationId(1));
    value.correlation.request_id = Some(RequestId("logical-one".into()));
    value.correlation.client_attempt = Some(ClientAttemptId::new());
    let mut filter = ObservationFilter { request: Some(RequestObservationId(1)), ..ObservationFilter::default() };
    assert!(filter.matches(&input));
    input.epoch.daemon.daemon_instance = "other".into();
    assert!(matches!(f.journal.sink().unwrap().record(input.clone()), RecordDisposition::Dropped(DropReason::RetiredEpoch)));
    filter.request = Some(RequestObservationId(2));
    assert!(!filter.matches(&input));
}

#[test]
fn late_report_keeps_claimed_origin_and_authenticated_current_submitter_separate() {
    use crate::observability::context::EvidenceValue;
    use crate::observability::runtime::{AcceptedPeer, ClientAttemptKind, ClientAttemptReport, ClientCallKnowledge, ConnectionId, ConnectionRef, ObservedProcess};
    let f = Fixture::new();
    let source_time = ObservationClock::new().now().unwrap();
    let mut historic = f.epoch.clone();
    historic.daemon.daemon_instance = "previous-daemon".into();
    let attempt = ClientAttemptId::new();
    let report = ClientAttemptReport {
        attempt,
        kind: ClientAttemptKind::Call,
        invocation: None,
        request_id: Some(RequestId("stable-logical-request".into())),
        requested_scope: f.epoch.daemon.scope.canonical_scope.clone(),
        resolved_scope: EvidenceValue::Present(f.epoch.daemon.scope.clone()),
        started_at: source_time.clone(),
        ended_at: source_time.clone(),
        stages: Vec::new(),
        omitted_stages: 0,
        last_ready: None,
        original_epoch: Some(historic.clone()),
        knowledge: ClientCallKnowledge::MayHaveBeenWritten,
        failure: None,
    };
    let peer = AcceptedPeer { connection: ConnectionRef { daemon: f.epoch.daemon.clone(), id: ConnectionId(1) }, process: ObservedProcess { pid: 42, incarnation: None }, observed_at: f.clock.now().unwrap() };
    let mut input =
        ObservationInput { epoch: f.epoch.clone(), source: EvidenceStamp { origin: EvidenceOrigin::ClientReported, at: source_time.clone() }, payload: ObservationPayload::ClientReported(ClientReportObservation { report, submitted_by: peer }) };
    assert!(input.validate());
    let mut filter = ObservationFilter { client_attempt: Some(attempt), ..ObservationFilter::default() };
    assert!(filter.matches(&input));
    filter.request = Some(RequestObservationId(1));
    assert!(!filter.matches(&input)); // A client claim is not a proven decoded server request.
    assert!(matches!(f.journal.sink().unwrap().record(input.clone()), RecordDisposition::Recorded(_)));
    let result = page(f.read(f.window(0, 1), 10));
    assert_eq!(result.records[0].input.source.at, source_time);
    assert_eq!(result.records[0].collected_at.clock, *f.clock.id());
    let ObservationPayload::ClientReported(observed) = &result.records[0].input.payload else { unreachable!() };
    assert_eq!(observed.report.original_epoch.as_ref(), Some(&historic));
    assert_eq!(observed.submitted_by.connection.daemon, f.epoch.daemon);

    let ObservationPayload::ClientReported(value) = &mut input.payload else { unreachable!() };
    value.submitted_by.connection.daemon = historic.daemon;
    assert!(!input.validate()); // Historic claims cannot authenticate the current submission.
}

#[test]
fn configuration_event_uses_original_stamp_and_exact_publication_epoch() {
    use crate::observability::configuration::{ConfigObservationEvent, ConfigSnapshotRef, ConfigStage, InvocationCaptureRef};
    use crate::observability::runtime::ClientInvocationId;
    let f = Fixture::new();
    let captured = ObservationClock::new().now().unwrap();
    let snapshot = ConfigSnapshotRef { epoch: f.epoch.clone(), sequence: 1 };
    let mut input = ObservationInput {
        epoch: f.epoch.clone(),
        source: EvidenceStamp { origin: EvidenceOrigin::ClientReported, at: captured.clone() },
        payload: ObservationPayload::Configuration(ConfigObservationEvent::SnapshotPublished {
            snapshot: snapshot.clone(),
            invocation: Some(InvocationCaptureRef { invocation: ClientInvocationId::new(), revision: 2 }),
            attempt: Some(ClientAttemptId::new()),
            stage: ConfigStage::InvocationInputs,
            origin: EvidenceOrigin::ClientReported,
            captured_at: captured,
        }),
    };
    assert!(input.validate());
    let filter = ObservationFilter { configuration: Some(snapshot), ..ObservationFilter::default() };
    assert!(filter.matches(&input));
    assert!(matches!(f.journal.sink().unwrap().record(input.clone()), RecordDisposition::Recorded(_)));
    input.source.origin = EvidenceOrigin::DaemonBoundary;
    assert!(!input.validate());
    input.source.origin = EvidenceOrigin::ClientReported;
    let ObservationPayload::Configuration(ConfigObservationEvent::SnapshotPublished { snapshot, .. }) = &mut input.payload else { unreachable!() };
    snapshot.epoch.database_generation = uuid::Uuid::new_v4().to_string();
    assert!(!input.validate());
}

#[test]
fn oversized_valid_projection_is_dropped_whole_before_sequence_assignment() {
    use crate::agents::attachments::{ActivationTarget, BindingId, ExecutionId, ParticipantRef};
    let f = Fixture::for_scope(ScopeRef { canonical_scope: std::env::temp_dir().join("s".repeat(3700)) });
    let mut input = f.input(1);
    let participant = ParticipantRef { canonical_name: "p".repeat(1024), incarnation_bits: 1 };
    input.payload = ObservationPayload::DomainCommitted(DomainCommitObservation {
        correlation: DomainCorrelation { request_id: Some(RequestId("r".repeat(1024))), actor: Some(participant.clone()), ..DomainCorrelation::default() },
        fact: DomainCommitFact::InboxPhaseStored {
            claim: InboxClaimRef {
                scope: f.epoch.daemon.scope.clone(),
                database_generation: f.epoch.database_generation.clone(),
                target: ActivationTarget { participant, execution: ExecutionId("e".repeat(1024)), binding: BindingId("b".repeat(1024)) },
                batch: BatchId("c".repeat(1024)),
            },
            phase: ClaimPhase::Issuing,
        },
    });
    assert!(input.validate());
    assert!(matches!(f.journal.sink().unwrap().record(input), RecordDisposition::Dropped(DropReason::Oversize)));
    let head = f.journal.describe().unwrap();
    assert_eq!(head.newest, 0);
    assert_eq!(head.losses.dropped.oversize, 1);
    assert_eq!(head.oldest, 1);
}

#[test]
fn byte_pressure_is_bounded_independently_of_count_and_page_encoding() {
    let f = Fixture::for_scope(ScopeRef { canonical_scope: std::env::temp_dir().join("s".repeat(3000)) });
    let sink = f.journal.sink().unwrap();
    for event in 1..=2100 {
        assert!(matches!(sink.record(f.input(event)), RecordDisposition::Recorded(_)));
    }
    let head = f.journal.describe().unwrap();
    assert!(head.losses.evicted.byte_pressure > 0);
    assert_eq!(head.losses.evicted.count_pressure, 0);
    {
        let state = f.journal.inner.state.lock().unwrap();
        let stream = state.as_ref().unwrap();
        assert!(stream.bytes <= MAX_RETAINED_BYTES);
        assert!(stream.records.len() < MAX_RECORDS);
    }
    let result = page(f.read(f.window(head.oldest - 1, head.newest), MAX_PAGE_RECORDS));
    assert!(result.records.len() < usize::from(MAX_PAGE_RECORDS)); // Encoded bytes end this page first.
    let bytes = encode_bounded(&ObservationResponse::Read(ObservationReadResult::Page(result.clone())), MAX_PAGE_BYTES).unwrap();
    assert!(bytes.len() <= MAX_PAGE_BYTES - 128);
    let continued = page(f.read(result.next, MAX_PAGE_RECORDS));
    assert_eq!(continued.records.first().unwrap().cursor.sequence, result.records.last().unwrap().cursor.sequence + 1);
}

#[test]
fn unpublished_generation_remains_unavailable_even_if_cleanup_is_contended() {
    let f = Fixture::new();
    let sink = f.journal.sink().unwrap();
    let held = f.journal.inner.state.lock().unwrap();
    assert!(matches!(f.journal.unpublish_epoch(), Err(ObservationReadError::Busy)));
    assert!(matches!(sink.record(f.input(1)), RecordDisposition::Dropped(DropReason::RetiredEpoch)));
    drop(held);
    assert!(matches!(f.journal.describe(), Err(ObservationReadError::GenerationUnavailable { .. })));
    let old_stream = sink.record(f.input(1));
    assert!(matches!(old_stream, RecordDisposition::Dropped(DropReason::RetiredEpoch)));
    f.journal.publish_epoch(f.epoch.clone()).unwrap();
    assert!(matches!(sink.record(f.input(1)), RecordDisposition::Dropped(DropReason::RetiredEpoch)));
    assert_eq!(f.append(2).sequence, 1);
}

#[test]
fn checked_counter_and_sequence_limits_never_wrap_or_reuse_an_identity() {
    let f = Fixture::new();
    let old = f.journal.describe().unwrap().stream;
    {
        let mut state = f.journal.inner.state.lock().unwrap();
        state.as_mut().unwrap().newest = u64::MAX - 1;
    }
    assert!(matches!(f.journal.sink().unwrap().record(f.input(1)), RecordDisposition::Dropped(DropReason::CollectorUnavailable)));
    assert!(matches!(f.journal.describe(), Err(ObservationReadError::Unavailable)));
    let next = f.journal.publish_epoch(f.epoch.clone()).unwrap();
    assert_ne!(old, next);
    let sink = f.journal.sink().unwrap();
    sink.note_drop(DropReason::Contended, u64::MAX);
    sink.note_drop(DropReason::Contended, 1);
    assert!(matches!(f.journal.describe(), Err(ObservationReadError::Unavailable)));
    let replacement = f.journal.replace_stream().unwrap();
    assert_ne!(next, replacement);
    assert!(matches!(sink.record(f.input(1)), RecordDisposition::Dropped(DropReason::RetiredEpoch)));
    assert_eq!(f.append(2).sequence, 1);
}

#[test]
fn poisoned_collector_is_unavailable_instead_of_retryable_contention() {
    let f = Fixture::new();
    let sink = f.journal.sink().unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _held = f.journal.inner.state.lock().unwrap();
        panic!("exercise collector poison boundary");
    }));
    assert!(result.is_err());
    assert!(matches!(f.journal.describe(), Err(ObservationReadError::Unavailable)));
    assert!(matches!(sink.record(f.input(1)), RecordDisposition::Dropped(DropReason::CollectorUnavailable)));
    assert!(matches!(f.journal.replace_stream(), Err(ObservationReadError::Unavailable)));
    assert!(matches!(f.journal.publish_epoch(f.epoch.clone()), Err(ObservationReadError::Unavailable)));
}

#[test]
fn close_is_terminal_even_when_a_lifecycle_owner_retains_the_journal() {
    let f = Fixture::new();
    f.journal.close();
    assert!(matches!(f.journal.publish_epoch(f.epoch.clone()), Err(ObservationReadError::Unavailable)));
    assert!(matches!(f.journal.replace_stream(), Err(ObservationReadError::Unavailable)));
    assert!(matches!(f.journal.sink(), Err(ObservationReadError::Unavailable)));
}

#[test]
fn suppression_never_appends_a_record_or_causes_recursive_instrumentation() {
    let f = Fixture::new();
    assert!(matches!(f.journal.suppress(), RecordDisposition::Suppressed));
    let head = f.journal.describe().unwrap();
    assert_eq!(head.newest, 0);
    assert_eq!(head.losses.suppressed, 1);
}
