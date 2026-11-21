//! Independent A4/A7/A8 persistence cases against real SQLite stores.
//! No mock of claim, cursor or attachment behavior is used here.

use super::*;
use crate::agents::attachments::{Attachment, AttachmentFailure, AttachmentIntent, AttachmentOwnerId, AttachmentRequest, AttachmentStore, ExecutionId, ProviderSessionKey, SqliteAttachmentStore};
use crate::db::AgentxDb;
use rusqlite::params;
use serde_json::json;

fn instance(db: &AgentxDb, name: &str, created_at: f64) -> ParticipantRef {
    let row = json!({"created_at": created_at, "status": "active", "tool": "codex", "last_event_id": 0});
    db.save_instance_named(name, row.as_object().unwrap()).unwrap();
    SqliteAttachmentStore::new(db).participant(name).unwrap()
}

fn session(id: &str) -> ProviderSessionKey {
    ProviderSessionKey { provider: "codex".into(), scope: "/fixture/codex-home".into(), session_id: id.into() }
}

fn attach(db: &AgentxDb, participant: ParticipantRef, id: &str, owner: &str) -> Attachment {
    SqliteAttachmentStore::new(db).attach(AttachmentRequest { execution: ExecutionId("backend-a".into()), owner: AttachmentOwnerId(owner.into()), session: session(id), intent: AttachmentIntent::Create { provisional: participant } }).unwrap()
}

fn queue(db: &AgentxDb, recipient: &str, text: &str) -> EventId {
    let data = json!({"from": "child", "text": text, "scope": "mentions", "mentions": [recipient], "delivered_to": [recipient], "sender_kind": "instance", "intent": "inform"});
    db.conn().execute("INSERT INTO events(timestamp,type,instance,data) VALUES('2026-11-07T14:00:00','message','child',?)", [data.to_string()]).unwrap();
    EventId(db.conn().last_insert_rowid())
}

fn native_batch(db: &AgentxDb, attachment: &Attachment) -> PreparedBatch {
    let mut inbox = SqlitePendingInbox::new(db);
    inbox.select_delivery_mode(&attachment.target, DeliveryMode::NativeInput).unwrap();
    let snapshot = inbox.snapshot(&attachment.target.participant).unwrap().unwrap();
    inbox.prepare(&attachment.target, &AttemptId("attempt-a".into()), &snapshot, InboxConsumer::NativeInput).unwrap()
}

fn receipt(attachment: &Attachment, batch: &PreparedBatch, evidence: DeliveryEvidence) -> DeliveryReceipt {
    DeliveryReceipt { target: attachment.target.clone(), batch: batch.id.clone(), through: batch.window.through, evidence }
}

fn native_evidence() -> DeliveryEvidence {
    DeliveryEvidence::ProviderInputRecorded { turn: TurnEvidence { opaque_turn_id: Some("turn-a".into()) } }
}

#[test]
fn product_input_requires_an_exact_managed_send_and_freezes_the_native_proof() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let generation = db.database_generation().unwrap();
    let participant = instance(&db, "lobo", 10.0);
    db.store_launch_context("lobo", r#"{"product_execution":"backend-a"}"#).unwrap();
    let attachment = attach(&db, participant.clone(), "root-a", "proxy-a");
    let target = &attachment.target;
    let mut inbox = SqlitePendingInbox::new(&db);
    assert!(inbox.select_delivery_mode(target, DeliveryMode::HookContext).is_err());
    inbox.select_delivery_mode(target, DeliveryMode::DedicatedProduct).unwrap();
    let first = queue(&db, "lobo", "one product message");
    let attempt = AttemptId("product-attempt".into());
    let product_source = |event: EventId, owner| {
        let source = ProductSend { participant: owner, execution: target.execution.clone(), request_id: uuid::Uuid::new_v4().to_string() };
        db.kv_set(&ProductSend::key(&generation, event.0), Some(&serde_json::to_string(&source).unwrap())).unwrap();
        source
    };
    assert!(inbox.prepare_product(target, &attempt, &generation).is_err(), "foreign send");
    assert!(inbox.outstanding(target).unwrap().is_none());
    product_source(first, ParticipantRef { incarnation_bits: 99, ..participant.clone() });
    assert!(inbox.prepare_product(target, &attempt, &generation).is_err(), "wrong incarnation");
    let source = product_source(first, participant.clone());
    let snapshot = inbox.snapshot(&participant).unwrap().unwrap();
    assert!(inbox.prepare(target, &attempt, &snapshot, InboxConsumer::ExplicitListen).is_err());
    let batch = inbox.prepare_product(target, &attempt, &generation).unwrap().unwrap();
    assert_eq!(batch.request_id, source.request_id);
    assert_eq!(batch.message_id, first.0.to_string());
    let second = queue(&db, "lobo", "later foreign message");
    inbox.mark_issuing(target, &batch.batch.id).unwrap();
    let mut proof = ProductInput { request_id: batch.request_id, message_id: second.0.to_string(), thread_id: "root-a".into(), turn_id: "turn-a".into(),
        item_id: "input-a".into(), payload_digest: ProductInput::digest(&batch.batch.formatted_context) };
    assert!(inbox.confirm(receipt(&attachment, &batch.batch, DeliveryEvidence::ProductInputRecorded(proof.clone()))).is_err());
    proof.message_id = batch.message_id;
    let mut wrong = proof.clone();
    wrong.thread_id = "wrong-root".into();
    assert!(inbox.confirm(receipt(&attachment, &batch.batch, DeliveryEvidence::ProductInputRecorded(wrong))).is_err());
    assert_eq!(inbox.delivered_through(&participant).unwrap(), EventId(0));
    let accepted = receipt(&attachment, &batch.batch, DeliveryEvidence::ProductInputRecorded(proof));
    assert_eq!(inbox.confirm(accepted.clone()).unwrap(), Confirmation::Advanced);
    let mut reopened = SqlitePendingInbox::new(&db);
    assert_eq!(reopened.confirm(accepted).unwrap(), Confirmation::AlreadyConfirmed);
    assert_eq!(reopened.delivered_through(&participant).unwrap(), first);
    assert!(reopened.prepare_product(target, &attempt, &generation).is_err(), "later foreign message");
}

#[test]
fn acceptance_a4_roots_and_owners_are_independent_and_live_ownership_cannot_be_stolen() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let a = attach(&db, instance(&db, "lobo", 10.0), "root-a", "proxy-a");
    let b = attach(&db, instance(&db, "vume", 11.0), "root-b", "proxy-b");
    assert_ne!(a.target.participant, b.target.participant);
    let mut store = SqliteAttachmentStore::new(&db);
    let competing = AttachmentRequest { execution: a.target.execution.clone(), owner: AttachmentOwnerId("proxy-b".into()), session: session("root-a"), intent: AttachmentIntent::Resume { provisional: None } };
    assert_eq!(store.attach(competing), Err(AttachmentFailure::OwnedByLiveController));
    store.retire_owner(&a.owner).unwrap();
    assert!(!store.validate(&a.target).unwrap());
    assert!(store.validate(&b.target).unwrap());
    assert_eq!(store.canonical_participant(&session("root-a")).unwrap(), Some(a.target.participant));
}

#[test]
fn acceptance_a7_cursor_commits_only_exact_claim_window_and_medium() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let attachment = attach(&db, instance(&db, "lobo", 10.0), "root-a", "proxy-a");
    let first = queue(&db, "lobo", "frozen batch alpha");
    // Unrelated messages and global events do not enlarge this recipient's
    // eligible window. A later arrival must survive confirmation of this batch.
    queue(&db, "vume", "unrelated");
    let batch = native_batch(&db, &attachment);
    let later = queue(&db, "lobo", "arrived after preparation");
    assert_eq!(batch.window.through, first);
    assert!(batch.formatted_context.contains("frozen batch alpha"));
    assert!(!batch.formatted_context.contains("unrelated"));
    let mut inbox = SqlitePendingInbox::new(&db);
    assert!(inbox.confirm(receipt(&attachment, &batch, DeliveryEvidence::HookOutputWritten)).is_err(), "wrong medium accepted native batch");
    assert_eq!(inbox.delivered_through(&attachment.target.participant).unwrap(), EventId(0));
    inbox.mark_issuing(&attachment.target, &batch.id).unwrap();
    let mut forged = receipt(&attachment, &batch, native_evidence());
    forged.through = later;
    assert!(!matches!(inbox.confirm(forged), Ok(Confirmation::Advanced | Confirmation::AlreadyConfirmed)));
    assert_eq!(inbox.delivered_through(&attachment.target.participant).unwrap(), EventId(0));
    assert_eq!(inbox.confirm(receipt(&attachment, &batch, native_evidence())).unwrap(), Confirmation::Advanced);
    assert_eq!(inbox.delivered_through(&attachment.target.participant).unwrap(), first);
    assert_eq!(inbox.confirm(receipt(&attachment, &batch, native_evidence())).unwrap(), Confirmation::AlreadyConfirmed);
    let pending = inbox.snapshot(&attachment.target.participant).unwrap().unwrap();
    assert_eq!(pending.through, later);
    assert_eq!(pending.count.get(), 1);
}

#[test]
fn acceptance_a8_all_consumers_contend_on_one_persisted_claim() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let attachment = attach(&db, instance(&db, "lobo", 10.0), "root-a", "proxy-a");
    queue(&db, "lobo", "one mailbox");
    let batch = native_batch(&db, &attachment);
    let other_connection = AgentxDb::open_at(db.path()).unwrap();
    let mut contender = SqlitePendingInbox::new(&other_connection);
    let snapshot = contender.snapshot(&attachment.target.participant).unwrap().unwrap();
    for consumer in [InboxConsumer::NativeInput, InboxConsumer::Hook, InboxConsumer::ExplicitListen] {
        assert!(contender.prepare(&attachment.target, &AttemptId("competing-attempt".into()), &snapshot, consumer).is_err(), "two consumers received same batch");
    }
    assert_eq!(contender.outstanding(&attachment.target).unwrap().unwrap().batch.id, batch.id);
    assert_eq!(contender.delivered_through(&attachment.target.participant).unwrap(), EventId(0));
}

#[test]
fn acceptance_a8_sqlite_concurrent_native_and_explicit_consumers_have_one_winner() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("agentx.db");
    let db = AgentxDb::open_at(&db_path).unwrap();
    let attachment = attach(&db, instance(&db, "lobo", 10.0), "root-a", "proxy-a");
    queue(&db, "lobo", "concurrent claim");
    let mut inbox = SqlitePendingInbox::new(&db);
    inbox.select_delivery_mode(&attachment.target, DeliveryMode::NativeInput).unwrap();
    let snapshot = inbox.snapshot(&attachment.target.participant).unwrap().unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let jobs: Vec<_> = [InboxConsumer::NativeInput, InboxConsumer::ExplicitListen]
        .into_iter()
        .enumerate()
        .map(|(index, consumer)| {
            let path = db_path.clone();
            let target = attachment.target.clone();
            let snapshot = snapshot.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let connection = AgentxDb::open_at(&path).unwrap();
                let mut inbox = SqlitePendingInbox::new(&connection);
                barrier.wait();
                inbox.prepare(&target, &AttemptId(format!("racing-{index}")), &snapshot, consumer)
            })
        })
        .collect();
    let outcomes: Vec<_> = jobs.into_iter().map(|job| job.join().unwrap()).collect();
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1, "claim race outcomes: {outcomes:?}");
    assert_eq!(inbox.delivered_through(&attachment.target.participant).unwrap(), EventId(0));
}

#[test]
fn acceptance_a8_unissued_release_is_legal_but_issued_unknown_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agentx.db");
    let db = AgentxDb::open_at(&path).unwrap();
    let attachment = attach(&db, instance(&db, "lobo", 10.0), "root-a", "proxy-a");
    queue(&db, "lobo", "possibly recorded");
    let first = native_batch(&db, &attachment);
    let batch = {
        let mut inbox = SqlitePendingInbox::new(&db);
        inbox.release_proven_unrecorded(&attachment.target, &first.id, NoInputEvidence::NeverIssued).unwrap();
        let batch = native_batch(&db, &attachment);
        inbox.mark_issuing(&attachment.target, &batch.id).unwrap();
        inbox.record_phase(&attachment.target, &batch.id, ClaimPhase::Uncertain).unwrap();
        assert!(inbox.release_proven_unrecorded(&attachment.target, &batch.id, NoInputEvidence::NeverIssued).is_err());
        batch
    };
    drop(db);
    let reopened = AgentxDb::open_at(&path).unwrap();
    let mut recovered = SqlitePendingInbox::new(&reopened);
    let outstanding = recovered.outstanding(&attachment.target).unwrap().unwrap();
    assert_eq!(outstanding.phase, ClaimPhase::Uncertain);
    assert_eq!(outstanding.batch.id, batch.id);
    assert!(recovered.release_proven_unrecorded(&attachment.target, &batch.id, NoInputEvidence::NeverIssued).is_err());
    assert_eq!(recovered.delivered_through(&attachment.target.participant).unwrap(), EventId(0));
}

#[test]
fn acceptance_a8_stale_receipt_cannot_commit_successor_and_explicit_recovery_keeps_lineage() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let original = attach(&db, instance(&db, "lobo", 10.0), "root-a", "proxy-a");
    let mail = queue(&db, "lobo", "old batch");
    let batch = native_batch(&db, &original);
    let mut inbox = SqlitePendingInbox::new(&db);
    inbox.mark_issuing(&original.target, &batch.id).unwrap();
    inbox.record_phase(&original.target, &batch.id, ClaimPhase::Uncertain).unwrap();
    let mut store = SqliteAttachmentStore::new(&db);
    store.retire(&original.target).unwrap();
    let successor = store.attach(AttachmentRequest { execution: ExecutionId("backend-b".into()), owner: AttachmentOwnerId("proxy-b".into()), session: session("root-a"), intent: AttachmentIntent::Resume { provisional: None } }).unwrap();
    assert_ne!(successor.target.binding, original.target.binding);
    assert_eq!(inbox.confirm(receipt(&original, &batch, native_evidence())).unwrap(), Confirmation::StaleFence);
    assert_eq!(inbox.delivered_through(&successor.target.participant).unwrap(), EventId(0));
    inbox.select_delivery_mode(&successor.target, DeliveryMode::NativeInput).unwrap();
    let snapshot = inbox.snapshot(&successor.target.participant).unwrap().unwrap();
    assert_eq!(inbox.prepare(&successor.target, &AttemptId("new-attempt".into()), &snapshot, InboxConsumer::NativeInput), Err(PrepareFailure::UnresolvedPredecessor));
    let evidence = ReconciliationEvidence::PreviouslyRecorded(receipt(&original, &batch, native_evidence()));
    assert_eq!(inbox.reconcile_predecessor(&batch.id, &successor.target, evidence).unwrap(), RecoveryOutcome::RecoveredDelivery);
    assert_eq!(inbox.delivered_through(&successor.target.participant).unwrap(), mail);
    assert!(inbox.snapshot(&successor.target.participant).unwrap().is_none());
}

#[test]
fn acceptance_a8_name_reuse_does_not_accept_old_receipt_or_forget_uncertain_claim() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let original = attach(&db, instance(&db, "lobo", 10.0), "root-a", "proxy-a");
    queue(&db, "lobo", "must not replay after name reuse");
    let batch = native_batch(&db, &original);
    let mut inbox = SqlitePendingInbox::new(&db);
    inbox.mark_issuing(&original.target, &batch.id).unwrap();
    inbox.record_phase(&original.target, &batch.id, ClaimPhase::Uncertain).unwrap();
    SqliteAttachmentStore::new(&db).retire(&original.target).unwrap();
    db.delete_instance("lobo").unwrap();
    let replacement = instance(&db, "lobo", 20.0);
    assert_ne!(replacement, original.target.participant);
    assert_eq!(inbox.confirm(receipt(&original, &batch, native_evidence())).unwrap(), Confirmation::StaleFence);
    assert_eq!(db.conn().query_row("SELECT last_event_id FROM instances WHERE name='lobo'", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
    // Restoring the known session must find durable lineage rather than using
    // only the fresh live instance row as if no possible input had occurred.
    let restored = SqliteAttachmentStore::new(&db).canonical_participant(&session("root-a"));
    assert_eq!(restored, Err(AttachmentFailure::StaleParticipant), "name reuse must not restore the old session into a different participant");
    let retained: String = db.conn().query_row("SELECT resolution FROM inbox_claims WHERE batch_id=?", [&batch.id.0], |row| row.get(0)).unwrap();
    assert_eq!(retained, "open", "name reuse discarded uncertain input history");
}

#[test]
fn acceptance_a8_query_failure_is_unavailable_and_never_empty_mail() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let attachment = attach(&db, instance(&db, "lobo", 10.0), "root-a", "proxy-a");
    queue(&db, "lobo", "pending before read failure");
    db.conn().execute("ALTER TABLE events RENAME TO fixture_events_unavailable", []).unwrap();
    let inbox = SqlitePendingInbox::new(&db);
    assert_eq!(inbox.snapshot(&attachment.target.participant), Err(InboxFailure::Unavailable));
    assert_eq!(db.conn().query_row("SELECT last_event_id FROM instances WHERE name=?", params!["lobo"], |row| row.get::<_, i64>(0)).unwrap(), 0);
}

#[test]
fn acceptance_a4_deleted_row_restores_canonical_identity_with_fresh_incarnation_and_cursor() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let original = attach(&db, instance(&db, "lobo", 10.0), "root-a", "proxy-a");
    db.update_instance_fields("lobo", json!({"parent_name":"parent","last_event_id":17}).as_object().unwrap()).unwrap();
    assert!(db.delete_instance("lobo").unwrap());
    // Durable snapshots can predate the current schema; restore only native fields.
    db.conn().execute("UPDATE participant_sessions SET snapshot=json_set(snapshot,'$.retired_metadata','historic')", []).unwrap();
    let mut store = SqliteAttachmentStore::new(&db);
    assert!(!store.validate(&original.target).unwrap());
    let restored = store.canonical_participant(&session("root-a")).unwrap().unwrap();
    assert_eq!(restored.canonical_name, original.target.participant.canonical_name);
    assert_ne!(restored.incarnation_bits, original.target.participant.incarnation_bits);
    let row = db.get_instance_full("lobo").unwrap().unwrap();
    assert_eq!(row.last_event_id, 17);
    assert_eq!(row.parent_name.as_deref(), Some("parent"));
    let saved: String = db.conn().query_row("SELECT snapshot FROM participant_sessions WHERE participant_name='lobo'", [], |row| row.get(0)).unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&saved).unwrap().get("retired_metadata").is_none());
    let resumed = store.attach(AttachmentRequest { execution: ExecutionId("backend-a".into()), owner: AttachmentOwnerId("proxy-b".into()), session: session("root-a"), intent: AttachmentIntent::Resume { provisional: None } }).unwrap();
    assert_eq!(resumed.target.participant, restored);
    assert_ne!(resumed.target.binding, original.target.binding);
}

#[test]
fn acceptance_a4_unregistered_resume_uses_only_explicit_reserved_identity() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let participant = instance(&db, "lobo", 10.0);
    let mut store = SqliteAttachmentStore::new(&db);
    assert!(store.canonical_participant(&session("unregistered-root")).unwrap().is_none());
    let unqualified = AttachmentRequest { execution: ExecutionId("backend-a".into()), owner: AttachmentOwnerId("proxy-a".into()), session: session("unregistered-root"), intent: AttachmentIntent::Resume { provisional: None } };
    assert!(store.attach(unqualified.clone()).is_err(), "resume guessed an arbitrary recipient");
    let qualified = AttachmentRequest { intent: AttachmentIntent::Resume { provisional: Some(participant.clone()) }, ..unqualified };
    let attached = store.attach(qualified).unwrap();
    assert_eq!(attached.target.participant, participant);
    assert_eq!(store.canonical_participant(&session("unregistered-root")).unwrap(), Some(participant));
}

#[test]
fn acceptance_a8_wrong_attempt_or_empty_history_reference_cannot_release_issued_mail() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let attachment = attach(&db, instance(&db, "lobo", 10.0), "root-a", "proxy-a");
    queue(&db, "lobo", "requires valid non-input evidence");
    let batch = native_batch(&db, &attachment);
    let mut inbox = SqlitePendingInbox::new(&db);
    inbox.mark_issuing(&attachment.target, &batch.id).unwrap();
    for proof in [NoInputEvidence::ProviderRejectedBeforeInput { attempt: AttemptId("wrong-attempt".into()) }, NoInputEvidence::OldExecutionEndedAndHistoryComplete { snapshot_reference: "  ".into() }] {
        assert!(inbox.release_proven_unrecorded(&attachment.target, &batch.id, proof).is_err());
    }
    assert_eq!(inbox.delivered_through(&attachment.target.participant).unwrap(), EventId(0));
    assert_eq!(inbox.outstanding(&attachment.target).unwrap().unwrap().batch.id, batch.id);
    inbox.release_proven_unrecorded(&attachment.target, &batch.id, NoInputEvidence::ProviderRejectedBeforeInput { attempt: AttemptId("attempt-a".into()) }).unwrap();
    assert!(inbox.outstanding(&attachment.target).unwrap().is_none());
}

#[test]
fn acceptance_a11_actual_v20_database_upgrades_in_place_without_losing_existing_provider_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agentx.db");
    let old = rusqlite::Connection::open(&path).unwrap();
    old.execute_batch(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/agentx_schema_v20.sql"))).unwrap();
    old.execute("INSERT INTO instances(name,created_at,status,tool,last_event_id) VALUES('lobo',10.0,'active','claude',7)", []).unwrap();
    let message = json!({"from":"child","text":"preserve before migration","scope":"mentions","mentions":["lobo"]}).to_string();
    old.execute("INSERT INTO events(id,timestamp,type,instance,data) VALUES(8,'2026-11-07','message','child',?)", [&message]).unwrap();
    old.execute("INSERT INTO process_bindings(process_id,session_id,instance_name,updated_at) VALUES('process-a','session-a','lobo',10.0)", []).unwrap();
    drop(old);
    let db = AgentxDb::open_at(&path).unwrap();
    assert_eq!(db.conn().query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0)).unwrap(), i64::from(crate::db::SCHEMA_VERSION));
    let row = db.get_instance_full("lobo").unwrap().unwrap();
    assert_eq!(row.tool, "claude");
    assert_eq!(row.last_event_id, 7);
    assert_eq!(row.created_at, 10.0);
    assert_eq!(db.conn().query_row("SELECT data FROM events WHERE id=8", [], |row| row.get::<_, String>(0)).unwrap(), message);
    assert_eq!(db.conn().query_row("SELECT instance_name FROM process_bindings WHERE process_id='process-a'", [], |row| row.get::<_, String>(0)).unwrap(), "lobo");
    for table in ["participant_sessions", "participant_attachments", "inbox_claims"] {
        assert_eq!(db.conn().query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get::<_, i64>(0)).unwrap(), 0);
    }
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().filter_map(Result::ok).filter(|entry| entry.file_name().to_string_lossy().contains("archive")).count(), 0, "additive migration archived the live database");
}

#[test]
fn acceptance_a1_policy_coalesces_notifications_while_accepted_work_has_not_yet_issued() {
    use crate::messaging::activation::{ActivationPolicy, Admission};
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let attachment = attach(&db, instance(&db, "lobo", 10.0), "root-a", "proxy-a");
    queue(&db, "lobo", "one attempt despite repeated notification");
    let mut inbox = SqlitePendingInbox::new(&db);
    inbox.select_delivery_mode(&attachment.target, DeliveryMode::NativeInput).unwrap();
    let mut policy = ActivationPolicy::default();
    let request = policy.prepare_request(&mut inbox, &attachment.target).unwrap().unwrap();
    policy.finish_admission(&mut inbox, &request, &Admission::Accepted { attempt: request.attempt.clone() }).unwrap();
    assert_eq!(inbox.outstanding(&attachment.target).unwrap().unwrap().phase, ClaimPhase::Prepared);
    for _ in 0..3 {
        assert!(policy.prepare_request(&mut inbox, &attachment.target).unwrap().is_none());
    }
    assert_eq!(inbox.delivered_through(&attachment.target.participant).unwrap(), EventId(0));
}

#[test]
fn acceptance_a2_policy_guaranteed_no_effect_defer_releases_prepared_batch_for_later() {
    use crate::messaging::activation::{ActivationPolicy, Admission, DeferReason};
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let attachment = attach(&db, instance(&db, "lobo", 10.0), "root-a", "proxy-a");
    // A released claim is newly formatted on reoffer. Hold one-time guidance
    // state fixed so full payload equality tests the same retained messages.
    db.kv_set("tip:lobo:recv:intent:inform", Some("1")).unwrap();
    queue(&db, "lobo", "deferred actual message");
    let mut inbox = SqlitePendingInbox::new(&db);
    inbox.select_delivery_mode(&attachment.target, DeliveryMode::NativeInput).unwrap();
    let mut policy = ActivationPolicy::default();
    let first = policy.prepare_request(&mut inbox, &attachment.target).unwrap().unwrap();
    policy.finish_admission(&mut inbox, &first, &Admission::Deferred(DeferReason::AwaitingApproval)).unwrap();
    assert!(inbox.outstanding(&attachment.target).unwrap().is_none());
    let later = policy.prepare_request(&mut inbox, &attachment.target).unwrap().unwrap();
    assert_ne!(first.attempt, later.attempt);
    assert_eq!(first.batch.formatted_context, later.batch.formatted_context);
    assert_eq!(inbox.delivered_through(&attachment.target.participant).unwrap(), EventId(0));
}

#[test]
fn acceptance_a8_policy_unknown_effect_blocks_reoffer_and_settled_never_confirms() {
    use crate::messaging::activation::{ActivationPolicy, Admission, DriverEvent};
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let attachment = attach(&db, instance(&db, "lobo", 10.0), "root-a", "proxy-a");
    queue(&db, "lobo", "uncertain input");
    let mut inbox = SqlitePendingInbox::new(&db);
    inbox.select_delivery_mode(&attachment.target, DeliveryMode::NativeInput).unwrap();
    let mut policy = ActivationPolicy::default();
    let request = policy.prepare_request(&mut inbox, &attachment.target).unwrap().unwrap();
    policy.finish_admission(&mut inbox, &request, &Admission::Accepted { attempt: request.attempt.clone() }).unwrap();
    policy.observe(&mut inbox, DriverEvent::Issued { target: attachment.target.clone(), attempt: request.attempt.clone() }).unwrap();
    policy.observe(&mut inbox, DriverEvent::OutcomeUnknown { target: attachment.target.clone(), attempt: request.attempt, detail: "response lost".into() }).unwrap();
    policy.observe(&mut inbox, DriverEvent::Settled { target: attachment.target.clone(), turn: TurnEvidence { opaque_turn_id: Some("completed-turn".into()) } }).unwrap();
    assert!(policy.prepare_request(&mut inbox, &attachment.target).unwrap().is_none());
    assert_eq!(inbox.outstanding(&attachment.target).unwrap().unwrap().phase, ClaimPhase::Uncertain);
    assert_eq!(inbox.delivered_through(&attachment.target.participant).unwrap(), EventId(0));
}
