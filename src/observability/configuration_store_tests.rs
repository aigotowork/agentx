//! Immutable retention, epoch, report and pressure scenarios for native integration.

use super::super::context::{EvidenceValue, ScopeRef};
use super::super::events::RecordDisposition;
use super::super::runtime::{AcceptedPeer, ClientInvocationId, ConnectionId, ConnectionRef, ObservedProcess};
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

fn fixture() -> (ConfigurationStore, ObservationEpoch) {
    let root = if cfg!(windows) { std::path::PathBuf::from(r"C:\fixtures\configuration") } else { std::path::PathBuf::from("/fixtures/configuration") };
    let scope = ScopeRef::from_canonical_scope(root).unwrap();
    let daemon = DaemonRef::from_lease(scope, "daemon-fixture".into()).unwrap();
    let epoch = ObservationEpoch::from_committed_generation(daemon.clone(), "11111111-1111-4111-8111-111111111111".into()).unwrap();
    let store = ConfigurationStore::new(daemon, ObservationClock::new(), None);
    store.rotate_epoch(epoch.clone()).unwrap();
    (store, epoch)
}
fn body(store: &ConfigurationStore, epoch: &ObservationEpoch, seconds: u64) -> ConfigSnapshotBody {
    let capture = InvocationCaptureHandle::begin(ClientInvocationId::new()).reference();
    let mut body = ConfigSnapshotBody::invocation(capture, store.inner.clock.now().unwrap(), RulesRevision::current("test-loaded-build"));
    body.stage = ConfigStage::ScopedEffective;
    body.purpose = ConfigPurpose::ActualResolution;
    body.resolution_scope = EvidenceValue::Present(epoch.daemon.scope.clone());
    body.fields.push(
        projected_field(
            ConfigFieldId::setting("timeout").unwrap(),
            EvidenceValue::Present(ConfigValue::Seconds(seconds)),
            FieldResolution::Effective { source: ConfigSourceRef { kind: ConfigSourceKind::InvocationEnvironment, read: None }, normalized: false },
        )
        .unwrap(),
    );
    body
}
fn publish(store: &ConfigurationStore, epoch: &ObservationEpoch, body: ConfigSnapshotBody) -> ConfigSnapshotRef {
    match store.record_resolved(OwnedConfigCapture { expected_epoch: epoch.clone(), body, provenance: OwnedConfigOrigin::ApplicationResolved }) {
        ConfigCaptureResult::Recorded(reference) => reference,
        ConfigCaptureResult::Skipped(reason) => panic!("unexpected capture skip {reason:?}"),
    }
}
fn peer(store: &ConfigurationStore, epoch: &ObservationEpoch, connection: u64, incarnation: Option<&str>) -> AcceptedPeer {
    AcceptedPeer { connection: ConnectionRef { daemon: epoch.daemon.clone(), id: ConnectionId(connection) }, process: ObservedProcess { pid: 321, incarnation: incarnation.map(str::to_owned) }, observed_at: store.inner.clock.now().unwrap() }
}
fn report(body: ConfigSnapshotBody) -> ConfigReport {
    ConfigReport { source: LocalConfigSnapshot { reference: LocalConfigSnapshotRef { capture: body.correlation.invocation.clone().unwrap(), sequence: 1 }, body } }
}

#[test]
fn two_client_values_have_one_retention_epoch_and_frozen_effective_evidence() {
    let (store, epoch) = fixture();
    let first = publish(&store, &epoch, body(&store, &epoch, 7));
    let second = publish(&store, &epoch, body(&store, &epoch, 31));
    let compared = store.compare(CompareConfig { left: first.clone(), right: second, fields: ConfigFieldSelection::Only(vec![ConfigFieldId::setting("timeout").unwrap()]) }).unwrap();
    assert_eq!(compared.epoch, epoch);
    match compared.result {
        CompareConfigResult::Compared { fields, basis } => {
            assert_eq!(fields[0].value, ValueRelation::Different);
            assert_eq!(basis.stage, ConfigStage::ScopedEffective);
            assert_eq!(basis.left_purpose, ConfigPurpose::ActualResolution);
        }
        _ => panic!("expected captured comparison"),
    }
    match store.lookup(first).unwrap().result {
        SnapshotLookup::Found(snapshot) => assert_eq!(snapshot.body.fields[0].value, EvidenceValue::Present(ConfigValue::Seconds(7))),
        _ => panic!("must retain original value"),
    }
}

#[test]
fn reset_checks_expected_epoch_under_admission_and_same_epoch_does_not_reset_allocator() {
    let (store, first_epoch) = fixture();
    let first = publish(&store, &first_epoch, body(&store, &first_epoch, 7));
    store.rotate_epoch(first_epoch.clone()).unwrap();
    let second = publish(&store, &first_epoch, body(&store, &first_epoch, 8));
    assert_eq!(second.sequence, first.sequence + 1);
    let old_prepared = body(&store, &first_epoch, 9);
    let mut next_epoch = first_epoch.clone();
    next_epoch.database_generation = "22222222-2222-4222-8222-222222222222".into();
    store.rotate_epoch(next_epoch.clone()).unwrap();
    assert!(matches!(store.record_resolved(OwnedConfigCapture { expected_epoch: first_epoch.clone(), body: old_prepared, provenance: OwnedConfigOrigin::ApplicationResolved }), ConfigCaptureResult::Skipped(CaptureSkipReason::StaleEpoch)));
    assert!(matches!(store.lookup(first), Err(ConfigurationQueryError::EpochChanged { .. })));
    assert_eq!(store.head(None).unwrap().skipped_other, 0, "old-era skip must not increment the successor's counters");
    let next = publish(&store, &next_epoch, body(&store, &next_epoch, 10));
    assert_eq!(next.sequence, 1);
    assert_ne!(next.epoch, first_epoch);
}

#[test]
fn reset_and_compare_never_return_mixed_epoch_rows() {
    let (store, epoch) = fixture();
    let left = publish(&store, &epoch, body(&store, &epoch, 7));
    let right = publish(&store, &epoch, body(&store, &epoch, 8));
    let mut next = epoch.clone();
    next.database_generation = "22222222-2222-4222-8222-222222222222".into();
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let reset = store.clone();
    let reset_barrier = Arc::clone(&barrier);
    let worker = std::thread::spawn(move || {
        reset_barrier.wait();
        reset.rotate_epoch(next).unwrap();
    });
    barrier.wait();
    let result = store.compare(CompareConfig { left, right, fields: ConfigFieldSelection::All });
    worker.join().unwrap();
    match result {
        Ok(read) => {
            assert_eq!(read.epoch, epoch);
            if let CompareConfigResult::Compared { basis, .. } = read.result {
                assert_eq!(basis.left.epoch, epoch);
                assert_eq!(basis.right.epoch, epoch);
            } else {
                panic!("retained same-era pair");
            }
        }
        Err(ConfigurationQueryError::EpochChanged { .. } | ConfigurationQueryError::Busy) => {}
        Err(error) => panic!("unexpected race result {error:?}"),
    }
}

#[test]
fn fifo_eviction_and_never_issued_are_distinct_without_reconstructing_values() {
    let (store, epoch) = fixture();
    store.inner.state.lock().unwrap().limits.snapshots = 1;
    let old = publish(&store, &epoch, body(&store, &epoch, 7));
    let current = publish(&store, &epoch, body(&store, &epoch, 44));
    assert!(matches!(store.lookup(old.clone()).unwrap().result, SnapshotLookup::Missing { reason: ConfigMissing::NoLongerRetained { .. }, .. }));
    let mut unissued = current.clone();
    unissued.sequence += 1;
    assert!(matches!(store.lookup(unissued).unwrap().result, SnapshotLookup::Missing { reason: ConfigMissing::NeverIssued, .. }));
    assert!(matches!(store.compare(CompareConfig { left: old, right: current, fields: ConfigFieldSelection::All }).unwrap().result, CompareConfigResult::Unavailable { left: SnapshotLookup::Missing { .. }, right: SnapshotLookup::Found(_) }));
}

#[test]
fn age_uses_admission_not_report_source_time_and_reads_do_not_extend_retention() {
    let (store, epoch) = fixture();
    let submitting_peer = peer(&store, &epoch, 1, Some("known-incarnation"));
    let mut original = body(&store, &epoch, 7);
    original.captured_at.monotonic_ms.0 = 0;
    let reference = store.record_report(epoch.clone(), &submitting_peer, report(original)).unwrap();
    assert!(matches!(store.lookup(reference.clone()).unwrap().result, SnapshotLookup::Found(_)));
    store.inner.state.lock().unwrap().limits.max_age_ms = 0;
    assert!(matches!(store.lookup(reference).unwrap().result, SnapshotLookup::Missing { reason: ConfigMissing::NoLongerRetained { .. }, .. }));
}

#[test]
fn client_report_preserves_claim_origin_and_rejects_authority_subjects_or_secret_values() {
    let (store, epoch) = fixture();
    let submitting_peer = peer(&store, &epoch, 1, Some("known-incarnation"));
    let original = body(&store, &epoch, 7);
    let original_time = original.captured_at.clone();
    let reference = store.record_report(epoch.clone(), &submitting_peer, report(original.clone())).unwrap();
    let duplicate = store.record_report(epoch.clone(), &submitting_peer, report(original.clone())).unwrap();
    assert_ne!(reference, duplicate);
    match store.lookup(reference).unwrap().result {
        SnapshotLookup::Found(snapshot) => {
            assert!(matches!(snapshot.provenance, ConfigProvenance::ClientReported { .. }));
            assert_eq!(snapshot.body.captured_at, original_time);
        }
        _ => panic!("retained safe report"),
    }
    let mut unsafe_body = original.clone();
    unsafe_body.fields.push(FieldEvidence { field: ConfigFieldId::setting("relay_psk").unwrap(), value: EvidenceValue::Present(ConfigValue::Text("DO_NOT_STORE_SECRET".into())), resolution: EvidenceValue::Withheld, steps: Vec::new() });
    assert!(matches!(store.record_report(epoch.clone(), &submitting_peer, report(unsafe_body)), Err(ConfigReportError::InvalidFields)));
    let mut hostile = report(original);
    hostile.source.body.subject = ConfigSubject::DaemonStartup { daemon: epoch.daemon.clone() };
    assert!(matches!(store.record_report(epoch, &submitting_peer, hostile), Err(ConfigReportError::InvalidSubject)));
}

#[test]
fn peer_quota_follows_known_incarnation_and_unknown_incarnation_is_connection_local() {
    let (store, epoch) = fixture();
    let original = body(&store, &epoch, 7);
    let known = peer(&store, &epoch, 1, Some("same-incarnation"));
    for _ in 0..64 {
        store.record_report(epoch.clone(), &known, report(original.clone())).unwrap();
    }
    let another_connection = peer(&store, &epoch, 2, Some("same-incarnation"));
    assert!(matches!(store.record_report(epoch.clone(), &another_connection, report(original.clone())), Err(ConfigReportError::PeerQuota)));
    let successor = peer(&store, &epoch, 3, Some("successor-incarnation"));
    assert!(store.record_report(epoch.clone(), &successor, report(original.clone())).is_ok());
    let unknown_a = peer(&store, &epoch, 4, None);
    let unknown_b = peer(&store, &epoch, 5, None);
    assert!(!same_report_peer(&ConfigProvenance::ClientReported { source: report(original.clone()).source.reference, peer: unknown_a }, &unknown_b));
}

#[test]
fn contention_cannot_wait_or_change_business_result_and_overflow_cannot_recycle_refs() {
    let (store, epoch) = fixture();
    let prepared = body(&store, &epoch, 7);
    let guard = store.inner.state.lock().unwrap();
    assert!(matches!(store.record_resolved(OwnedConfigCapture { expected_epoch: epoch.clone(), body: prepared.clone(), provenance: OwnedConfigOrigin::ApplicationResolved }), ConfigCaptureResult::Skipped(CaptureSkipReason::Pressure)));
    assert!(matches!(store.head(None), Err(ConfigurationQueryError::Busy)));
    drop(guard);
    store.inner.state.lock().unwrap().last_issued = u64::MAX;
    assert!(matches!(store.record_resolved(OwnedConfigCapture { expected_epoch: epoch.clone(), body: prepared, provenance: OwnedConfigOrigin::ApplicationResolved }), ConfigCaptureResult::Skipped(CaptureSkipReason::ObservationFailed)));
    assert!(matches!(store.rotate_epoch(epoch), Err(ConfigurationQueryError::Unavailable)));
}

struct ReentrantSink {
    store: Mutex<Option<ConfigurationStore>>,
    called: AtomicU64,
}
impl ObservationSink for ReentrantSink {
    fn record(&self, _: ObservationInput) -> RecordDisposition {
        if let Some(store) = self.store.lock().unwrap().as_ref() {
            assert!(store.head(None).is_ok(), "event sink must run outside evidence lock");
        }
        self.called.fetch_add(1, Ordering::Relaxed);
        RecordDisposition::Suppressed
    }
    fn note_drop(&self, _: super::super::events::DropReason, _: u64) {}
}
#[test]
fn event_publication_releases_store_lock_before_calling_sink() {
    let (store, epoch) = fixture();
    let sink = Arc::new(ReentrantSink { store: Mutex::new(Some(store.clone())), called: AtomicU64::new(0) });
    store.set_sink(Some(sink.clone()));
    publish(&store, &epoch, body(&store, &epoch, 7));
    assert_eq!(sink.called.load(Ordering::Relaxed), 1);
    store.set_sink(None);
    *sink.store.lock().unwrap() = None;
}

#[test]
fn finite_facade_preserves_scope_epoch_selection_and_oversize_error_branches() {
    let (store, epoch) = fixture();
    let accepted = peer(&store, &epoch, 1, Some("known-incarnation"));
    assert!(matches!(store.handle(&accepted, ConfigurationRequest::Head { expected: Some(epoch.clone()) }), Ok(ConfigurationResponse::Head(_))));
    let mut wrong_scope = body(&store, &epoch, 7);
    wrong_scope.resolution_scope = EvidenceValue::Present(ScopeRef::from_canonical_scope(epoch.daemon.scope.canonical_scope.join("other-root")).unwrap());
    assert!(matches!(store.handle(&accepted, ConfigurationRequest::ReportSafeCapture { expected_epoch: epoch.clone(), report: report(wrong_scope) }), Err(ConfigurationError::Report(ConfigReportError::InvalidSubject))));

    let valid = body(&store, &epoch, 7);
    let reference = match store.handle(&accepted, ConfigurationRequest::ReportSafeCapture { expected_epoch: epoch.clone(), report: report(valid.clone()) }).unwrap() {
        ConfigurationResponse::Reported(reference) => reference,
        _ => panic!("report facade response"),
    };
    assert!(matches!(
        store.handle(&accepted, ConfigurationRequest::Compare(CompareConfig { left: reference.clone(), right: reference.clone(), fields: ConfigFieldSelection::Only(Vec::new()) })),
        Err(ConfigurationError::Query(ConfigurationQueryError::InvalidSelection))
    ));
    let mut earlier = reference.clone();
    earlier.epoch.database_generation = "22222222-2222-4222-8222-222222222222".into();
    assert!(matches!(store.compare(CompareConfig { left: earlier, right: reference, fields: ConfigFieldSelection::All }).unwrap().result, CompareConfigResult::Incomparable(ComparisonBoundary::DifferentEpoch)));

    store.inner.state.lock().unwrap().limits.snapshot_bytes = 1;
    assert!(matches!(store.record_report(epoch.clone(), &accepted, report(valid)), Err(ConfigReportError::Oversize)));
    assert_eq!(store.head(None).unwrap().last_issued, 1, "failed admission allocates no identity");
}

#[test]
fn counter_exhaustion_fences_capture_until_a_genuinely_new_epoch() {
    let (store, epoch) = fixture();
    store.inner.state.lock().unwrap().skipped_other = u64::MAX;
    let mut invalid = body(&store, &epoch, 7);
    invalid.purpose = ConfigPurpose::ValidatedHostReport;
    let result = store.record_resolved(OwnedConfigCapture { expected_epoch: epoch.clone(), body: invalid, provenance: OwnedConfigOrigin::ApplicationResolved });
    assert!(matches!(result, ConfigCaptureResult::Skipped(_)));
    assert!(matches!(store.rotate_epoch(epoch.clone()), Err(ConfigurationQueryError::Unavailable)));
    let mut successor = epoch;
    successor.database_generation = "22222222-2222-4222-8222-222222222222".into();
    store.rotate_epoch(successor.clone()).unwrap();
    assert_eq!(publish(&store, &successor, body(&store, &successor, 7)).sequence, 1);
}

#[test]
fn byte_pressure_retires_a_fifo_prefix_without_exceeding_the_payload_budget() {
    let (store, epoch) = fixture();
    store.inner.state.lock().unwrap().limits.payload_bytes = MAX_SNAPSHOT_BYTES as u64;
    let mut large = body(&store, &epoch, 7);
    for context in [ContextFieldId::Cwd, ContextFieldId::RequestedScope, ContextFieldId::CanonicalScope, ContextFieldId::ProviderHome, ContextFieldId::EffectiveWorkingDirectory, ContextFieldId::EffectiveDevRoot] {
        large.fields.push(projected_field(ConfigFieldId::Context(context), EvidenceValue::Present(ConfigValue::Path(std::path::PathBuf::from("x".repeat(3900)))), FieldResolution::InputOnly).unwrap());
    }
    let first = publish(&store, &epoch, large.clone());
    let second = publish(&store, &epoch, large);
    let head = store.head(None).unwrap();
    assert_eq!(head.retained_count, 1);
    assert!(head.retained_payload_bytes <= head.limits.payload_bytes);
    assert_eq!(head.retired_through, first.sequence);
    assert!(matches!(store.lookup(second).unwrap().result, SnapshotLookup::Found(_)));
}

#[test]
fn participant_and_host_projections_need_their_owned_intake_ports() {
    let (store, epoch) = fixture();
    let accepted = peer(&store, &epoch, 1, Some("known-incarnation"));
    let participant = crate::agents::attachments::ParticipantRef { canonical_name: "fixture".into(), incarnation_bits: 1 };
    let mut projected = body(&store, &epoch, 7);
    projected.subject = ConfigSubject::Participant { participant: participant.clone() };
    projected.stage = ConfigStage::ParticipantState;
    projected.purpose = ConfigPurpose::DomainProjection;
    projected.fields = vec![
        projected_field(
            ConfigFieldId::Participant(ParticipantFieldId::Timeout),
            EvidenceValue::Present(ConfigValue::Seconds(172800)),
            FieldResolution::Effective { source: ConfigSourceRef { kind: ConfigSourceKind::DomainRecord, read: None }, normalized: false },
        )
        .unwrap(),
    ];
    assert!(matches!(store.record_report(epoch.clone(), &accepted, report(projected.clone())), Err(ConfigReportError::InvalidSubject)));
    assert!(matches!(store.record_resolved(OwnedConfigCapture { expected_epoch: epoch.clone(), body: projected.clone(), provenance: OwnedConfigOrigin::StoredDomainProjection }), ConfigCaptureResult::Recorded(_)));
    projected.subject = ConfigSubject::Host { participant, execution: None };
    projected.stage = ConfigStage::HostReported;
    projected.purpose = ConfigPurpose::ValidatedHostReport;
    assert!(matches!(store.record_report(epoch.clone(), &accepted, report(projected.clone())), Err(ConfigReportError::InvalidSubject)));
    let reference = store.record_host_report(epoch, &accepted, HostConfigReport { body: projected }).unwrap();
    match store.lookup(reference).unwrap().result {
        SnapshotLookup::Found(snapshot) => assert!(matches!(snapshot.provenance, ConfigProvenance::HostReported { peer } if peer == accepted)),
        _ => panic!("validated host port retains its exact accepted peer"),
    }
}
