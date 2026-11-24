use super::*;
use crate::events::service::{Request, Response};
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};
use serde_json::json;

fn db(path: &std::path::Path) -> AgentxDb {
    AgentxDb::open_at(path).unwrap()
}

fn participant(db: &AgentxDb, name: &str, created_at: f64) -> ParticipantRef {
    db.save_instance_named(name, json!({"tool":"codex","created_at":created_at,"status":"active"}).as_object().unwrap()).unwrap();
    ParticipantRef { canonical_name: name.to_owned(), incarnation_bits: created_at.to_bits() }
}

fn context(name: Option<&str>, id: &str) -> RequestContext {
    RequestContext {
        caller: CallerContext { request_id: RequestId(id.into()), invocation: InvocationContext { explicit_name: name.map(str::to_owned), ..Default::default() }, deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid: std::process::id(), incarnation: None },
        daemon_instance: "reminder-fixture".into(),
        database_generation: "reminder-fixture-generation".into(),
    }
}

fn add(db: &AgentxDb, actor: &str, id: &str, target: &ParticipantRef, due: i64, body: &str) -> ReminderCreated {
    let Response::ReminderCreated(created) = super::super::store::handle(db, &context(Some(actor), id), Request::CreateReminder { target: target.clone(), due_at_ms: due, message: body.into() }).unwrap() else { panic!("expected reminder receipt") };
    created
}

#[test]
fn create_replay_and_cancel_preserve_terminal_outcomes() {
    let root = tempfile::tempdir().unwrap();
    let db = db(&root.path().join("agentx.db"));
    let creator = participant(&db, "luna", 1.0);
    let target = participant(&db, "peso", 2.0);
    let due = now_ms() + 10_000;
    let first = add(&db, "luna", "request", &target, due, "check");
    let second = add(&db, "luna", "new request", &target, due, "check");
    assert_ne!(first.reminder.id, second.reminder.id);
    assert!(!first.already_exists);
    let id = &first.reminder.id;
    assert_eq!(show(&db, id).unwrap().state, ReminderState::Pending);
    assert!(matches!(cancel(&db, &context(None, "anonymous cancellation"), id), Err(ServiceError::IdentityRequired(_))));
    assert!(cancel(&db, &context(Some("peso"), "target cancellation"), id).unwrap());
    assert_eq!(deliver(&db, id, due).unwrap(), None);
    let replay = add(&db, "luna", "request", &target, due, "check");
    assert!(replay.already_exists);
    assert_eq!(replay.reminder.id, *id);
    assert_eq!(replay.reminder.state, ReminderState::Canceled);
    assert!(matches!(create(&db, &context(Some("luna"), "request"), target.clone(), due, "different text".into()), Err(ServiceError::Conflict(_))));
    assert!(!list(&db, false, None).unwrap().reminders.iter().any(|r| r.id == *id));
    assert!(list(&db, true, None).unwrap().reminders.iter().any(|r| r.id == *id));

    let second_id = second.reminder.id;
    assert_eq!(deliver(&db, &second_id, due).unwrap(), Some(ReminderState::Enqueued));
    assert!(matches!(cancel(&db, &context(Some(&creator.canonical_name), "too late"), &second_id), Err(ServiceError::Conflict(_))));
    assert!(show(&db, &second_id).unwrap().delivery_event_id.is_some());
}

#[test]
fn rollback_leaves_pending_and_a_poison_row_does_not_block_later_batches() {
    let root = tempfile::tempdir().unwrap();
    let db = db(&root.path().join("agentx.db"));
    let actor = participant(&db, "luna", 1.0);
    let due = now_ms() + 10_000;
    let bad = add(&db, "luna", "bad", &actor, due, "rollback").reminder.id;
    db.conn().execute_batch(&format!("CREATE TRIGGER fail_completion BEFORE UPDATE ON reminders WHEN old.id='{bad}' BEGIN SELECT RAISE(ABORT,'injected completion failure'); END;")).unwrap();
    // The append succeeds inside the scope, then the completion write fails.
    assert!(deliver(&db, &bad, due).is_err());
    assert_eq!(db.get_last_event_id(), 0);
    assert_eq!(show(&db, &bad).unwrap().state, ReminderState::Pending);
    for i in 0..BATCH {
        add(&db, "luna", &format!("batch {i}"), &actor, due + 1, "healthy");
    }
    let mut scheduler = Schedule::new(&db);
    assert!(scheduler.process_due(&db, due + 1).unwrap());
    let done = || list(&db, true, None).unwrap().reminders.iter().filter(|r| r.state == ReminderState::Enqueued).count();
    assert!(done() < BATCH as usize);
    for _ in 0..4 {
        scheduler.process_due(&db, due + 1).unwrap();
    }
    assert_eq!(done(), BATCH as usize);
    assert_eq!(show(&db, &bad).unwrap().state, ReminderState::Pending);
    db.conn().execute_batch("DROP TRIGGER fail_completion").unwrap();
    scheduler.process_due(&db, due + 1).unwrap();
    assert_eq!(done(), BATCH as usize + 1);
    assert_eq!(db.try_get_unread_messages("luna").unwrap().len(), BATCH as usize + 1);
}

#[test]
fn fence_applies_to_indexed_and_legacy_mailbox_counts_and_delivery_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let db = db(&root.path().join("agentx.db"));
    // Negative f64 exercises bits outside a SQLite signed integer.
    let original = participant(&db, "peso", -1.0);
    participant(&db, "rune", 2.0);
    let due = now_ms() + 10_000;
    let id = add(&db, "peso", "exact", &original, due, "@rune also sees this?").reminder.id;
    assert_eq!(deliver(&db, &id, due).unwrap(), Some(ReminderState::Enqueued));
    let event = show(&db, &id).unwrap().delivery_event_id.unwrap();
    let read = |name: &str| -> Vec<i64> { db.unread_messages_up_to(name, 1).unwrap().iter().filter_map(|m| m.event_id).collect() };
    let count = |name| {
        let row = db.get_instance_full(name).unwrap().unwrap();
        crate::service::queries_store::listing::try_get_unread_counts_batch(&db, &[row]).unwrap()[name]
    };
    assert_eq!(read("peso"), [event]);
    assert_eq!(count("peso"), 1);
    assert!(!db.has_pending("rune"));
    assert!(db.start_mailbox_routes().unwrap());
    assert!(!db.backfill_mailbox_routes().unwrap());
    assert!(db.start_conversation_routes().unwrap());
    assert!(!db.backfill_conversation_routes().unwrap());
    assert_eq!(read("peso"), [event]);
    assert_eq!(count("peso"), 1);
    assert!(db.has_direct_unread("peso") && db.has_pending("peso"));

    assert!(db.delete_instance("peso").unwrap());
    participant(&db, "peso", 3.0);
    assert!(!db.has_direct_unread("peso") && !db.has_pending("peso"));
    assert_eq!(count("peso"), 0);
    let report = crate::messaging::delivery_report::report(&db, event).unwrap();
    assert_eq!(report.recipients[0].mailbox, crate::messaging::delivery_report::MailboxProgress::TargetRetired);
    assert_eq!(report.recipients[0].participant, Some(original));
    let ordinary = db.insert_event_row("message", "rune", &json!({"from":"rune","text":"ordinary","scope":"mentions","mentions":["peso"],"delivered_to":["peso"]}), None).unwrap();
    assert_eq!(read("peso"), [ordinary]);
    assert_eq!(count("peso"), 1);
    let snapshot = crate::messaging::delivery_store::snapshot(&db, &context(Some("peso"), "snapshot")).unwrap();
    assert_eq!(snapshot.pending.iter().filter_map(|m| m.event_id).collect::<Vec<_>>(), [ordinary]);
    assert_eq!(db.try_get_unread_messages("peso").unwrap().len(), 1);
}

#[test]
fn schema_29_migrates_without_losing_subscriptions_and_reset_clears_reminders() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("agentx.db");
    {
        let db = db(&path);
        db.kv_set("events_sub:fixture", Some(r#"{"id":"sub-fixture"}"#)).unwrap();
        db.conn().execute_batch("DROP TABLE reminders; PRAGMA user_version=29;").unwrap();
    }
    let db = db(&path);
    assert_eq!(db.conn().query_row("PRAGMA user_version", [], |row| row.get::<_, i32>(0)).unwrap(), crate::db::SCHEMA_VERSION);
    assert!(db.kv_get("events_sub:fixture").unwrap().is_some());
    let actor = participant(&db, "luna", 1.0);
    add(&db, "luna", "reset", &actor, now_ms() + 30_000, "clear");
    db.with_write_scope(|| db.clear_managed_state_for_reset()).unwrap();
    assert!(list(&db, true, None).unwrap().reminders.is_empty());
    assert!(db.kv_get("events_sub:fixture").unwrap().is_none(), "reset still clears the legacy managed records");
}

#[test]
fn daemon_restart_rewakes_exact_unread_target_without_second_append() {
    use std::net::TcpListener;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("agentx.db");
    let wake = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    wake.set_nonblocking(true).unwrap();
    let event = {
        let db = db(&path);
        let actor = participant(&db, "luna", 1.0);
        let due = now_ms() + 10_000;
        let id = add(&db, "luna", "wake", &actor, due, "persisted before restart").reminder.id;
        // Leave no endpoint at append time. Register it afterward, simulating
        // a persisted unread row whose commit-side wake never ran.
        assert_eq!(deliver(&db, &id, due).unwrap(), Some(ReminderState::Enqueued));
        db.upsert_notify_endpoint("luna", "hook", wake.local_addr().unwrap().port()).unwrap();
        show(&db, &id).unwrap().delivery_event_id.unwrap()
    };
    let _daemon = crate::daemon::test_support::TestDaemon::start(root.path(), &path);
    assert!(wake.accept().is_ok(), "daemon startup did not nudge the original endpoint");
    let db = db(&path);
    assert_eq!(db.get_last_event_id(), event);
    assert_eq!(db.try_get_unread_messages("luna").unwrap().len(), 1);
    assert!(matches!(wake.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock));
}

#[test]
fn history_page_retains_equal_due_time_rows_at_the_boundary() {
    let root = tempfile::tempdir().unwrap();
    let db = db(&root.path().join("agentx.db"));
    let actor = participant(&db, "luna", 1.0);
    let due = now_ms() + 30_000;
    let mut ids = Vec::new();
    db.with_write_scope(|| {
        for i in 0..101 {
            let id = add(&db, "luna", &format!("request {i}"), &actor, due, "same due").reminder.id;
            ids.push(id);
        }
        Ok(())
    })
    .unwrap();
    ids.sort();
    let first = list(&db, true, None).unwrap();
    let second = list(&db, true, first.next.as_ref()).unwrap();
    assert!(first.next.is_some() && second.next.is_none());
    assert_eq!(first.reminders.into_iter().chain(second.reminders).map(|r| r.id).collect::<Vec<_>>(), ids);
}

#[test]
fn conversation_seeks_verify_exact_fence_even_inside_a_matching_time_window() {
    let root = tempfile::tempdir().unwrap();
    let db = db(&root.path().join("agentx.db"));
    let target = participant(&db, "luna", 1.0);
    let due = now_ms() + 30_000;
    let id = add(&db, "luna", "conversation", &target, due, "for the original instance").reminder.id;
    assert_eq!(deliver(&db, &id, due).unwrap(), Some(ReminderState::Enqueued));
    let old = show(&db, &id).unwrap().delivery_event_id.unwrap();
    let ordinary = db.insert_event_row("message", "rune", &json!({"from":"rune","text":"by name","delivered_to":["luna"]}), None).unwrap();
    let replaced = ParticipantRef { canonical_name: "luna".into(), incarnation_bits: 2.0_f64.to_bits() };
    // Both messages fall in the time window. Only the reminder must use the
    // exact identity rather than relying on the timestamps alone.
    let read = |target: &ParticipantRef| {
        let ctx = context(None, "conversation");
        let start = "1970-01-01T00:00:00";
        let forward = super::super::store::conversation_page(&db, &ctx, target, start, 0, Some(ordinary), 10).unwrap().events;
        let backward = super::super::store::conversation_history(&db, target, start, ordinary, 10).unwrap().0;
        assert_eq!(forward, backward);
        forward.iter().map(|event| event["id"].as_i64().unwrap()).collect::<Vec<_>>()
    };
    assert_eq!(read(&target), [old, ordinary]);
    assert_eq!(read(&replaced), [ordinary]);
    assert!(db.start_conversation_routes().unwrap());
    assert!(!db.backfill_conversation_routes().unwrap());
    assert_eq!(read(&target), [old, ordinary]);
    assert_eq!(read(&replaced), [ordinary]);
}
