//! Actual loader/domain reads feed buffered projections; no telemetry re-read.

use super::*;
use crate::observability::{configuration::*, context::*, runtime::*};

fn fixture(root: &std::path::Path, generation: String) -> (ActiveObservation, ConfigurationStore) {
    let daemon = DaemonRef::from_lease(ScopeRef::from_canonical_scope(root.canonicalize().unwrap()).unwrap(), "context-fixture".into()).unwrap();
    let epoch = ObservationEpoch::from_committed_generation(daemon.clone(), generation).unwrap();
    let clock = ObservationClock::new();
    let journal = crate::observability::events::ObservationJournal::new(daemon.clone(), clock.clone()).unwrap();
    journal.publish_epoch(epoch.clone()).unwrap();
    let sink = journal.sink().unwrap();
    let runtime = RuntimeRegistry::new(daemon.clone(), clock.clone(), None).unwrap();
    let store = ConfigurationStore::new(daemon, clock.clone(), None);
    store.rotate_epoch(epoch.clone()).unwrap();
    let origin = crate::observability::runtime::OriginRef { invocation: InvocationCaptureHandle::begin(ClientInvocationId::new()).reference(), attempt: ClientAttemptId::new() };
    (
        ActiveObservation {
            epoch,
            clock,
            request: None,
            peer: None,
            accepted: None,
            origin: Some(origin),
            request_id: RequestId("config-use".into()),
            runtime: std::sync::Arc::new(runtime),
            configuration: std::sync::Arc::new(store.clone()),
            sink,
            pending_configuration: Default::default(),
        },
        store,
    )
}

fn snapshots(store: &ConfigurationStore) -> Vec<ConfigSnapshot> {
    let head = store.head(None).unwrap();
    (1..=head.last_issued)
        .filter_map(|sequence| match store.lookup(ConfigSnapshotRef { epoch: head.epoch.clone(), sequence }).unwrap().result {
            SnapshotLookup::Found(snapshot) => Some(snapshot),
            _ => None,
        })
        .collect()
}

#[test]
fn buffered_file_declaration_and_actual_resolution_survive_a_later_file_edit() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    std::fs::write(&path, "[preferences]\ntimeout=9\nhints='private-original-hints'\n").unwrap();
    let (active, store) = fixture(root.path(), uuid::Uuid::new_v4().to_string());
    let guard = active.enter();
    let (config, _) = crate::config::AgentxConfig::load_for_scope_with_errors(&root.path().canonicalize().unwrap(), &std::collections::HashMap::from([("AGENTX_TIMEOUT".into(), "17".into())]));
    assert_eq!(config.timeout, 17);
    assert_eq!(store.head(None).unwrap().last_issued, 0);
    std::fs::write(&path, "[preferences]\ntimeout=77\n").unwrap();
    drop(guard);
    let rows = snapshots(&store);
    let declaration = rows.iter().find(|row| row.body.stage == ConfigStage::ScopedDeclarations).unwrap();
    let resolved = rows.iter().find(|row| row.body.stage == ConfigStage::ScopedEffective).unwrap();
    let timeout = ConfigFieldId::setting("timeout").unwrap();
    assert_eq!(declaration.body.fields.iter().find(|field| field.field == timeout).unwrap().value, EvidenceValue::Present(ConfigValue::Seconds(9)));
    assert_eq!(resolved.body.fields.iter().find(|field| field.field == timeout).unwrap().value, EvidenceValue::Present(ConfigValue::Seconds(17)));
    assert_eq!(declaration.body.source_reads[0].read_revision, resolved.body.source_reads[0].read_revision);
    assert!(!serde_json::to_string(&rows).unwrap().contains("private-original-hints"));
}

#[test]
fn participant_and_persistent_dev_root_use_their_actual_read_provenance() {
    let root = tempfile::tempdir().unwrap();
    let db = crate::db::AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    db.conn().execute("INSERT INTO instances(name, created_at, wait_timeout, subagent_timeout, hints) VALUES ('luna', 42.0, 25, 7, 'private-hints')", []).unwrap();
    let dev_root = root.path().join("dev");
    db.kv_set(crate::db::DEV_ROOT_KV_KEY, Some(dev_root.to_str().unwrap())).unwrap();
    let generation = db.database_generation().unwrap();
    let (active, store) = fixture(root.path(), generation.clone());
    let context = RequestContext {
        caller: CallerContext { request_id: RequestId("actual-domain-read".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: Some(generation.clone()) },
        process: CallerProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
        daemon_instance: active.epoch.daemon.daemon_instance.clone(),
        database_generation: generation,
    };
    let guard = active.enter();
    crate::agents::service::handle(&db, &context, crate::agents::service::Request::Record { name: "luna".into() }).unwrap();
    crate::service::maintenance_store::handle(&db, &context, crate::service::maintenance::Request::DevRoot).unwrap();
    assert_eq!(store.head(None).unwrap().last_issued, 0);
    drop(guard);
    let rows = snapshots(&store);
    let participant = rows.iter().find(|row| row.body.stage == ConfigStage::ParticipantState).unwrap();
    assert!(matches!(participant.provenance, ConfigProvenance::StoredDomainProjection));
    assert!(participant.body.source_reads.is_empty());
    let timeout = participant.body.fields.iter().find(|field| field.field == ConfigFieldId::Participant(ParticipantFieldId::Timeout)).unwrap();
    assert_eq!(timeout.value, EvidenceValue::Present(ConfigValue::Seconds(25)));
    let dev = rows.iter().find(|row| row.body.fields.iter().any(|field| field.field == ConfigFieldId::Context(ContextFieldId::EffectiveDevRoot))).unwrap();
    assert_eq!(dev.body.source_reads[0].resource, SourceResource::PersistentDevRoot);
    assert_eq!(dev.body.fields[0].value, EvidenceValue::Present(ConfigValue::Path(dev_root)));
    let encoded = serde_json::to_string(&rows).unwrap();
    assert!(!encoded.contains("private-hints"));
}
