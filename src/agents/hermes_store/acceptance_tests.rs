//! Independent Hermes registry transactions; no Provider files or processes.
use super::*;
use crate::agents::hermes_service::Add;
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};

fn with_db(body: impl FnOnce(&AgentxDb, RequestContext, std::path::PathBuf)) {
    let root = tempfile::Builder::new().prefix("agentx-hermes-native-").tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    let pid = std::process::id();
    let context = RequestContext {
        caller: CallerContext { request_id: RequestId("hermes-add".into()), invocation: InvocationContext { cwd: root.path().to_path_buf(), ..Default::default() }, deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid, incarnation: crate::sys::process::identity(pid) },
        daemon_instance: "hermes-fixture".into(),
        database_generation: db.database_generation().unwrap(),
    };
    let home = root.path().join("uncreated-provider-profile");
    body(&db, context, home.clone());
    assert!(!home.exists(), "daemon registry created Provider files");
}

fn request(home: &Path) -> Request {
    Request::Add(Add {
        home: home.to_path_buf(),
        session_key: "telegram:fixture-chat".into(),
        platform: "telegram".into(),
        chat: "fixture chat".into(),
        origin: json!({"fixture":true}),
        name: Some("lobo".into()),
        session_id: Some("hermes-session-one".into()),
    })
}

fn active(response: Response) -> ActiveBridge {
    let Response::Active(entry) = response else {
        panic!("active Hermes response");
    };
    entry
}

fn next(context: &RequestContext, id: &str) -> RequestContext {
    let mut context = context.clone();
    context.caller.request_id = RequestId(id.into());
    context
}

fn bridge_actor(context: &RequestContext, entry: &ActiveBridge, id: &str) -> RequestContext {
    let mut context = next(context, id);
    context.caller.invocation.process_binding_id = Some(entry.bridge.token.clone());
    context.caller.invocation.explicit_name = Some(entry.bridge.name.clone());
    context
}

#[test]
fn add_converges_and_replays_but_rejects_changed_intent_and_peer() {
    with_db(|db, context, home| {
        let input = request(&home);
        let first = active(handle(db, &context, input.clone()).unwrap());
        let watermark = db.get_last_event_id();
        let replay = active(handle(db, &context, input.clone()).unwrap());
        let converged = active(handle(db, &next(&context, "same-chat-new-request"), input.clone()).unwrap());
        assert_eq!(first.bridge, replay.bridge);
        assert_eq!(first.participant, replay.participant);
        assert_eq!(first.bridge, converged.bridge);
        assert_eq!(first.participant, converged.participant);
        assert_eq!(db.iter_instances_full().unwrap().len(), 1);
        assert_eq!(stored_bridges(db).unwrap().len(), 1);
        assert_eq!(db.get_last_event_id(), watermark);
        let Request::Add(mut changed) = input.clone() else { unreachable!() };
        changed.chat = "changed retained intent".into();
        assert!(matches!(handle(db, &context, Request::Add(changed)), Err(ServiceError::Conflict(_))));
        let mut stolen = context.clone();
        stolen.process.incarnation = Some("different-authenticated-peer".into());
        assert!(matches!(handle(db, &stolen, input), Err(ServiceError::Conflict(_))));
        assert_eq!(db.get_last_event_id(), watermark);
    });
}

#[test]
fn add_required_acceptance_failure_rolls_back_registry_identity_and_events() {
    with_db(|db, context, home| {
        let before = db.get_last_event_id();
        db.conn().execute_batch("CREATE TRIGGER hermes_accept_fail BEFORE INSERT ON kv WHEN NEW.key LIKE 'participant_request:%' BEGIN SELECT RAISE(ABORT, 'fixture acceptance failure'); END;").unwrap();
        assert!(handle(db, &context, request(&home)).is_err());
        assert!(stored_bridges(db).unwrap().is_empty());
        assert!(db.iter_instances_full().unwrap().is_empty());
        assert_eq!(db.get_last_event_id(), before);
        db.conn().execute_batch("DROP TRIGGER hermes_accept_fail").unwrap();
        let accepted = active(handle(db, &context, request(&home)).unwrap());
        assert_eq!(accepted.participant.canonical_name, "lobo");
    });
}

#[test]
fn bind_and_visibility_replay_preserve_later_state_and_exact_token_fence() {
    with_db(|db, context, home| {
        let entry = active(handle(db, &context, request(&home)).unwrap());
        let actor = bridge_actor(&context, &entry, "bind-first");
        let bind = Request::BindSession { participant: entry.participant.clone(), session_id: "session-two".into() };
        handle(db, &actor, bind.clone()).unwrap();
        handle(db, &next(&actor, "bind-newer"), Request::BindSession { participant: entry.participant.clone(), session_id: "session-three".into() }).unwrap();
        handle(db, &actor, bind.clone()).unwrap();
        assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().session_id.as_deref(), Some("session-three"));
        let visible = Request::Visibility { participant: entry.participant.clone(), visibility: Visibility::Full };
        let visibility_context = next(&actor, "visibility-first");
        handle(db, &visibility_context, visible.clone()).unwrap();
        handle(db, &next(&actor, "visibility-newer"), Request::Visibility { participant: entry.participant.clone(), visibility: Visibility::Off }).unwrap();
        assert_eq!(active(handle(db, &visibility_context, visible).unwrap()).bridge.visibility, Visibility::Off);
        let mut stolen = next(&actor, "wrong-token");
        stolen.caller.invocation.process_binding_id = Some("visible-name-is-not-the-token".into());
        assert!(matches!(handle(db, &stolen, bind.clone()), Err(ServiceError::StaleFence)));
        db.update_instance_fields("lobo", json!({"created_at":99.0}).as_object().unwrap()).unwrap();
        assert!(matches!(handle(db, &actor, bind), Err(ServiceError::StaleFence)));
    });
}

#[test]
fn remove_rolls_back_descendants_then_replays_frozen_effects_and_rejects_successor() {
    with_db(|db, context, home| {
        let entry = active(handle(db, &context, request(&home)).unwrap());
        db.save_instance_named("nava", json!({"tool":"codex","created_at":23.0,"session_id":"child-session","parent_name":"lobo","parent_session_id":"hermes-session-one","status":"listening"}).as_object().unwrap()).unwrap();
        db.set_process_binding("child-token", "child-session", "nava").unwrap();
        let context = next(&context, "remove-with-child");
        let request = Request::Remove { expected: RegistryEntry { bridge: entry.bridge.clone(), participant: Some(entry.participant.clone()) } };
        let watermark = db.get_last_event_id();
        db.conn().execute_batch("CREATE TRIGGER hermes_parent_fail BEFORE DELETE ON instances WHEN OLD.name='lobo' BEGIN SELECT RAISE(ABORT, 'fixture parent delete'); END;").unwrap();
        assert!(handle(db, &context, request.clone()).is_err());
        assert_eq!(db.iter_instances_full().unwrap().len(), 2);
        assert_eq!(stored_bridges(db).unwrap(), vec![entry.bridge.clone()]);
        assert_eq!(db.get_process_binding("child-token").unwrap().as_deref(), Some("nava"));
        assert_eq!(db.get_last_event_id(), watermark);
        db.conn().execute_batch("DROP TRIGGER hermes_parent_fail").unwrap();
        let removed = handle(db, &context, request.clone()).unwrap();
        let Response::Removed { effects, .. } = &removed else {
            panic!("removed outcome");
        };
        assert_eq!(effects.iter().map(|effect| effect.record.name.as_str()).collect::<Vec<_>>(), vec!["nava", "lobo"]);
        assert!(db.iter_instances_full().unwrap().is_empty());
        assert!(stored_bridges(db).unwrap().is_empty());
        let after = db.get_last_event_id();
        assert_eq!(serde_json::to_value(&removed).unwrap(), serde_json::to_value(handle(db, &context, request.clone()).unwrap()).unwrap());
        assert_eq!(db.get_last_event_id(), after);
        db.save_instance_named("lobo", json!({"tool":"hermes","created_at":99.0,"status":"active"}).as_object().unwrap()).unwrap();
        assert!(matches!(handle(db, &context, request), Err(ServiceError::StaleFence)));
    });
}

#[test]
fn sync_preserves_zero_cursor_and_session_but_drops_explicitly_killed_registry() {
    with_db(|db, context, home| {
        let first = active(handle(db, &context, request(&home)).unwrap());
        db.log_event("life", "unrelated", &json!({"action":"fixture-progress"})).unwrap();
        db.update_instance_fields("lobo", json!({"last_event_id":0}).as_object().unwrap()).unwrap();
        let stop = |participant, reason: &str| {
            crate::agents::service::handle(db, &context, crate::agents::service::Request::Stop { participant, reason: reason.into() }).unwrap();
        };
        stop(first.participant.clone(), "session_end");
        let Response::Synced(recovered) = handle(db, &next(&context, "recover-stopped"), Request::Sync { home: home.clone() }).unwrap() else {
            panic!("synced registry");
        };
        assert_eq!(recovered.len(), 1);
        assert!(recovered[0].recreated);
        assert_ne!(first.participant, recovered[0].participant);
        assert_eq!(first.bridge.token, recovered[0].bridge.token);
        let row = db.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!(row.last_event_id, 0);
        assert_eq!(row.session_id.as_deref(), Some("hermes-session-one"));
        stop(recovered[0].participant.clone(), "killed");
        let Response::Synced(remaining) = handle(db, &next(&context, "sync-killed"), Request::Sync { home }).unwrap() else {
            panic!("synced registry");
        };
        assert!(remaining.is_empty());
        assert!(stored_bridges(db).unwrap().is_empty());
    });
}

#[test]
fn grant_and_gateway_bookkeeping_leave_provider_files_to_the_host() {
    with_db(|db, context, home| {
        let grant = |operation| handle(db, &context, Request::Grant { home: home.clone(), operation }).unwrap();
        assert!(matches!(grant(GrantOperation::Read), Response::GrantOwned(false)));
        assert!(matches!(grant(GrantOperation::RecordOwned), Response::GrantOwned(true)));
        assert!(matches!(grant(GrantOperation::Read), Response::GrantOwned(true)));
        assert!(matches!(grant(GrantOperation::ClearOwned), Response::GrantOwned(false)));
        handle(db, &context, Request::GatewayVersion { home: home.clone(), current: false }).unwrap();
        let key = format!("hermes_gateway_stale:{}", home.display());
        assert!(db.kv_get(&key).unwrap().is_some());
        handle(db, &context, Request::GatewayVersion { home, current: true }).unwrap();
        assert!(db.kv_get(&key).unwrap().is_none());
        assert!(db.iter_instances_full().unwrap().is_empty());
    });
}
