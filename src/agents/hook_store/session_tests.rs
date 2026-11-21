//! Native hook-session admission regressions; no Provider or host effects.
use super::*;
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};

fn fixture(body: impl FnOnce(&AgentxDb, RequestContext)) {
    let root = tempfile::Builder::new().prefix("agentx-hook-session-admission-").tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    let pid = std::process::id();
    let context = RequestContext {
        caller: CallerContext {
            request_id: RequestId(uuid::Uuid::new_v4().to_string()),
            invocation: InvocationContext {
                cwd: root.path().to_path_buf(),
                process_binding_id: Some("launch-process".into()),
                environment: [("AGENTX_LAUNCHED".into(), "1".into()), ("AGENTX_TOOL".into(), "qoder".into()), ("AGENTX_INSTANCE_NAME".into(), "root".into())].into(),
                ..Default::default()
            },
            deadline_unix_ms: None,
            expected_database_generation: None,
        },
        process: CallerProcess { pid, incarnation: crate::sys::process::identity(pid) },
        daemon_instance: "session-admission-test".into(),
        database_generation: db.database_generation().unwrap(),
    };
    body(&db, context);
}

fn seed(db: &AgentxDb, name: &str, created_at: f64, session: Option<&str>) -> ParticipantRef {
    db.save_instance_named(name, json!({"created_at":created_at,"tool":"qoder","session_id":session,"status":"listening","status_context":"hook:idle","last_event_id":0}).as_object().unwrap()).unwrap();
    if let Some(session) = session {
        db.set_session_binding(session, name).unwrap();
    }
    participant(&db.get_instance_full(name).unwrap().unwrap())
}

fn input(session: &str, create_if_unbound: bool) -> HookSession {
    HookSession { tool: "qoder".into(), session_id: session.into(), transcript_path: Some("/fixture/transcript.jsonl".into()), directory: Some("/fixture/hook-project".into()), create_if_unbound }
}

/// Compare every row plus binding timestamps and the append-only event frontier.
fn snapshot(db: &AgentxDb) -> serde_json::Value {
    let mut rows = db.iter_instances_full().unwrap();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    let processes = db
        .conn()
        .prepare("SELECT process_id,session_id,instance_name,updated_at FROM process_bindings ORDER BY process_id")
        .unwrap()
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, f64>(3)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let sessions = db
        .conn()
        .prepare("SELECT session_id,instance_name,created_at FROM session_bindings ORDER BY session_id")
        .unwrap()
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, f64>(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let events: i64 = db.conn().query_row("SELECT count(*) FROM events", [], |row| row.get(0)).unwrap();
    json!({"rows":rows,"processes":processes,"sessions":sessions,"events":events,"event_frontier":db.get_last_event_id()})
}

fn rejected_without_changes(db: &AgentxDb, context: &RequestContext, hook: HookSession) {
    let before = snapshot(db);
    let result = observe_session(db, context, hook);
    assert!(matches!(result, Err(ServiceError::StaleFence | ServiceError::IdentityRequired(_) | ServiceError::Forbidden(_))), "late launched hook must reject, got {result:?}");
    assert_eq!(snapshot(db), before, "rejection must preserve rows, status, metadata, bindings and events");
}

#[test]
fn launched_hook_cannot_create_after_a_missing_deleted_or_dangling_process_binding() {
    for binding in ["missing", "deleted", "dangling"] {
        for create in [false, true] {
            fixture(|db, context| {
                if binding != "missing" {
                    seed(db, "root", 1.0, None);
                    db.set_process_binding("launch-process", "", "root").unwrap();
                    db.delete_instance("root").unwrap();
                    if binding == "deleted" {
                        db.delete_process_binding("launch-process").unwrap();
                    }
                }
                rejected_without_changes(db, &context, input("late-session", create));
                assert!(db.iter_instances_full().unwrap().is_empty());
            });
        }
    }
}

#[test]
fn a_late_launched_hook_cannot_select_or_rebind_a_namesake_successor_by_session() {
    for create in [false, true] {
        fixture(|db, context| {
            seed(db, "root", 1.0, Some("retired-session"));
            db.set_process_binding("launch-process", "retired-session", "root").unwrap();
            db.delete_instance("root").unwrap();
            db.delete_process_binding("launch-process").unwrap();
            let successor = seed(db, "root", 2.0, Some("successor-session"));
            db.set_process_binding("successor-process", "successor-session", "root").unwrap();
            rejected_without_changes(db, &context, input("successor-session", create));
            assert_eq!(participant(&db.get_instance_full("root").unwrap().unwrap()), successor);
            assert!(db.get_process_binding("launch-process").unwrap().is_none());
            assert_eq!(db.get_process_binding("successor-process").unwrap().as_deref(), Some("root"));
        });
    }
}

#[test]
fn killed_marker_blocks_hook_rebinding_and_ordinary_status_revival() {
    fixture(|db, context| {
        let fence = seed(db, "root", 1.0, Some("original-session"));
        db.set_process_binding("launch-process", "original-session", "root").unwrap();
        db.mark_killed("root", "bigboss").unwrap();
        for session in ["original-session", "different-session"] {
            for create in [false, true] {
                rejected_without_changes(db, &context, input(session, create));
            }
        }
        let before = snapshot(db);
        for status in ["active", "listening", "blocked"] {
            let request = crate::agents::service::Request::Status { participant: fence.clone(), status: status.into(), context: "hook:user_prompt_submit".into(), detail: "late callback".into() };
            assert!(matches!(crate::agents::service_store::handle(db, &context, request), Err(ServiceError::StaleFence)));
            assert_eq!(snapshot(db), before, "status callback must preserve the operator kill marker and event frontier");
        }
        assert_eq!(db.get_instance_full("root").unwrap().unwrap().status_context, "exit:killed");
    });
}

#[test]
fn a_retained_hook_session_reply_cannot_admit_a_now_killed_participant() {
    fixture(|db, context| {
        seed(db, "root", 1.0, None);
        db.set_process_binding("launch-process", "", "root").unwrap();
        let hook = input("first-session", true);
        let observed = observe_session(db, &context, hook.clone()).unwrap();
        db.mark_killed(&observed.participant.canonical_name, "bigboss").unwrap();
        rejected_without_changes(db, &context, hook);
    });
}

#[test]
fn current_launched_first_bind_and_session_switch_keep_native_identity_policy() {
    fixture(|db, mut context| {
        let original = seed(db, "root", 1.0, None);
        db.set_process_binding("launch-process", "", "root").unwrap();
        let first = observe_session(db, &context, input("first-session", true)).unwrap();
        assert_eq!(first.participant, original);
        assert_eq!(first.record.session_id.as_deref(), Some("first-session"));
        assert_eq!(first.record.transcript_path, "/fixture/transcript.jsonl");
        assert_eq!(first.record.directory, "/fixture/hook-project");
        assert_eq!(db.get_session_binding("first-session").unwrap().as_deref(), Some("root"));
        assert_eq!(db.get_process_binding("launch-process").unwrap().as_deref(), Some("root"));
        let next = seed(db, "next", 2.0, Some("next-session"));
        context.caller.request_id = RequestId(uuid::Uuid::new_v4().to_string());
        let switched = observe_session(db, &context, input("next-session", true)).unwrap();
        assert_eq!(switched.participant, next);
        assert_eq!(db.get_process_binding("launch-process").unwrap().as_deref(), Some("next"));
        assert_eq!(db.get_session_binding("next-session").unwrap().as_deref(), Some("next"));
        let previous = db.get_instance_full("root").unwrap().unwrap();
        assert_eq!(previous.status, "inactive");
        assert_eq!(previous.status_context, "exit:session_switch");
        assert!(db.get_session_binding("first-session").unwrap().is_none());
        assert_eq!(db.iter_instances_full().unwrap().len(), 2);
    });
}

#[test]
fn genuinely_unlaunched_create_if_unbound_still_adopts_the_native_session() {
    fixture(|db, mut context| {
        context.caller.invocation.environment.remove("AGENTX_LAUNCHED");
        context.caller.invocation.environment.remove("AGENTX_INSTANCE_NAME");
        let adopted = observe_session(db, &context, input("adhoc-session", true)).unwrap();
        let name = adopted.participant.canonical_name;
        assert_eq!(adopted.record.tool, "qoder");
        assert_eq!(adopted.record.session_id.as_deref(), Some("adhoc-session"));
        assert_eq!(adopted.record.directory, "/fixture/hook-project");
        assert_eq!(db.get_process_binding("launch-process").unwrap().as_deref(), Some(name.as_str()));
        assert_eq!(db.get_session_binding("adhoc-session").unwrap().as_deref(), Some(name.as_str()));
        context.caller.request_id = RequestId(uuid::Uuid::new_v4().to_string());
        let again = observe_session(db, &context, input("adhoc-session", true)).unwrap();
        assert_eq!(again.participant.canonical_name, name);
        assert_eq!(db.iter_instances_full().unwrap().len(), 1);
    });
}
