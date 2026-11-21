//! Development cases exercise native reservation and exact retained projections,
//! not paid Provider execution or the independently owned acceptance harness.
use super::*;
use crate::agents::{launch_service as launch, lifecycle_service as life};
use crate::management::contract::{AgentRef, LaunchRelations, Request as Read, Response as ReadResult};
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};
use serde_json::json;
fn context(db: &AgentxDb, actor: Option<&str>) -> RequestContext {
    RequestContext {
        caller: CallerContext {
            request_id: RequestId(uuid::Uuid::new_v4().to_string()),
            invocation: InvocationContext { cwd: db.path().parent().unwrap().to_owned(), explicit_name: actor.map(str::to_owned), ..Default::default() },
            deadline_unix_ms: None,
            expected_database_generation: None,
        },
        process: CallerProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
        daemon_instance: "fixture".into(),
        database_generation: db.database_generation().unwrap(),
    }
}
fn input(db: &AgentxDb, name: &str) -> launch::Reserve {
    launch::Reserve {
        name: Some(name.into()),
        tool: "qoder".into(),
        title: None,
        labels: Default::default(),
        directory: db.path().parent().unwrap().to_owned(),
        process_binding: uuid::Uuid::new_v4().to_string(),
        prior_session: None,
        resume_cursor: None,
        background: true,
        launch_args: vec![],
        launch_profile: None,
        collaboration: None,
    }
}
fn launch(db: &AgentxDb, name: &str, actor: Option<&str>) -> ParticipantRef {
    let launch::Response::Reserved(result) = super::super::launch_store::handle(db, &context(db, actor), launch::Request::Reserve(input(db, name))).unwrap() else { panic!("reservation") };
    result.participant
}
fn read(db: &AgentxDb, participant: &ParticipantRef, after: Option<String>, limit: u32) -> Result<LaunchRelations, ServiceError> {
    let ReadResult::LaunchRelations(result) =
        crate::management::store::handle(db, &context(db, None), Read::LaunchRelations { agent: AgentRef { name: participant.canonical_name.clone(), incarnation: Some(participant.incarnation_bits.to_string()), stopped_event: None }, after, limit })?
    else {
        panic!("relations")
    };
    Ok(result)
}
fn stop(db: &AgentxDb, participant: &ParticipantRef) {
    let snapshot = db.get_instance_snapshot(&participant.canonical_name).unwrap();
    db.with_write_scope(|| {
        db.log_event("life", &participant.canonical_name, &json!({"action":"stopped","snapshot":snapshot}))?;
        db.delete_instance(&participant.canonical_name)?;
        Ok(())
    })
    .unwrap();
}
#[test]
fn launch_relations_reservation_then_child_instruction_fence() {
    use crate::agents::context_service::{Request as ContextRequest, Response as ContextResponse};
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let parent = launch(&db, "lobo", None);
    for (name, actor) in [("mavu", None), ("nigo", Some("lobo"))] {
        let original = context(&db, actor);
        let input = input(&db, name);
        let binding = input.process_binding.clone();
        let launch::Response::Reserved(reserved) = super::super::launch_store::handle(&db, &original, launch::Request::Reserve(input)).unwrap() else { panic!("reservation") };
        let read_request = || ContextRequest::ReadInstructions { participant: reserved.participant.clone() };
        // The original external/parent caller must not read the child's state.
        assert!(super::super::context_store::handle(&db, &original, read_request()).is_err());
        let env = std::collections::HashMap::from([("AGENTX_PROCESS_ID".into(), binding)]);
        let child = crate::runtime::context::AgentxContext::from_env(&env, dir.path().to_owned());
        let mut child_request = context(&db, None);
        child_request.caller.invocation = crate::runtime::host_client::invocation(&child);
        assert!(matches!(super::super::context_store::handle(&db, &child_request, read_request()).unwrap(), ContextResponse::Instructions(None)));
        let freeze = ContextRequest::FreezeInstructions { participant: reserved.participant.clone(), instructions: crate::runtime::agent_context::FrozenInstructions { text: name.into() } };
        assert!(matches!(super::super::context_store::handle(&db, &child_request, freeze).unwrap(), ContextResponse::Instructions(Some(_))));
        let relations = read(&db, &reserved.participant, None, 20).unwrap();
        assert_eq!(relations.origin, if actor.is_some() { "agent" } else { "external" });
        assert_eq!(relations.upstream.map(|upstream| upstream.incarnation), actor.map(|_| parent.incarnation_bits.to_string()));
    }
}
#[test]
fn launch_relations_native_chain_stopped_namesake_and_retention() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let a = launch(&db, "lobo", None);
    let b = launch(&db, "mavu", Some("lobo"));
    let d = launch(&db, "nigo", Some("mavu"));
    assert_eq!(read(&db, &a, None, 20).unwrap().origin, "external");
    assert_eq!(read(&db, &d, None, 20).unwrap().upstream.unwrap().incarnation, b.incarnation_bits.to_string());
    stop(&db, &b);
    let successor = launch(&db, "mavu", None);
    let downstream = read(&db, &a, None, 20).unwrap().downstream;
    assert_eq!(downstream.len(), 1);
    assert_eq!(downstream[0].incarnation, b.incarnation_bits.to_string());
    assert!(downstream[0].agent.as_ref().unwrap().agent.stopped_event.is_some());
    assert!(read(&db, &successor, None, 20).unwrap().downstream.is_empty());
    db.conn().execute("DELETE FROM events WHERE instance='mavu' AND json_extract(data,'$.action')='stopped'", []).unwrap();
    assert_eq!(read(&db, &a, None, 20).unwrap().downstream[0].availability, "unavailable");
}
#[test]
fn launch_relations_reservation_atomic_and_paged_subject_fences() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let a = launch(&db, "lobo", None);
    db.conn().execute_batch("CREATE TRIGGER relation_failure BEFORE INSERT ON agent_creations BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap();
    assert!(super::super::launch_store::handle(&db, &context(&db, Some("lobo")), launch::Request::Reserve(input(&db, "mavu"))).is_err());
    assert!(db.get_instance_full("mavu").unwrap().is_none());
    db.conn().execute_batch("DROP TRIGGER relation_failure;").unwrap();
    launch(&db, "mavu", Some("lobo"));
    launch(&db, "nigo", Some("lobo"));
    let page = read(&db, &a, None, 1).unwrap();
    assert_eq!(page.downstream.len(), 1);
    let next = page.next.unwrap();
    assert_eq!(read(&db, &a, Some(next.clone()), 1).unwrap().downstream.len(), 1);
    let other = launch(&db, "pivu", None);
    assert!(read(&db, &other, Some(next), 1).is_err());
    assert!(read(&db, &a, None, 101).is_err());
    db.with_write_scope(|| db.clear_managed_state_for_reset()).unwrap();
    assert_eq!(db.conn().query_row::<i64, _, _>("SELECT COUNT(*) FROM agent_creations", [], |r| r.get(0)).unwrap(), 0);
}
#[test]
fn launch_relations_real_resume_does_not_reparent_and_fork_uses_initiator() {
    let dir = tempfile::tempdir().unwrap();
    let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
    let a = launch(&db, "lobo", None);
    let b = launch(&db, "mavu", Some("lobo"));
    let operator = launch(&db, "pivu", None);
    db.update_instance_fields("mavu", json!({"session_id":"session-fixture"}).as_object().unwrap()).unwrap();
    let saved_cursor = db.get_instance_full("mavu").unwrap().unwrap().last_event_id;
    stop(&db, &b);
    let resolve = |fork| {
        let life::Response::ResumeSelected(result) = super::super::lifecycle_store::handle(&db, &context(&db, Some("pivu")), life::Request::ResolveResume { target: "mavu".into(), expected: None, fork }).unwrap() else { panic!("source") };
        let life::ResumeSelection::Tracked { fence, .. } = result.selection else { panic!("tracked") };
        fence
    };
    let source = resolve(false);
    let mut launch = input(&db, "mavu");
    launch.prior_session = Some(source.session_id.clone());
    launch.resume_cursor = Some(saved_cursor);
    let life::Response::ResumeReserved(resumed) = super::super::lifecycle_store::handle(&db, &context(&db, Some("pivu")), life::Request::ReserveResume { source, launch }).unwrap() else { panic!("resumed") };
    assert_ne!(resumed.participant, b);
    let resumed_relations = read(&db, &resumed.participant, None, 20).unwrap();
    assert_eq!(resumed_relations.upstream.unwrap().name, "lobo");
    assert!(resumed_relations.fork_source.is_none());
    assert!(read(&db, &operator, None, 20).unwrap().downstream.is_empty());
    assert_eq!(read(&db, &a, None, 20).unwrap().downstream.len(), 1);
    let source = resolve(true);
    let life::Response::ResumeFork(source) = super::super::lifecycle_store::handle(&db, &context(&db, Some("pivu")), life::Request::ReserveResumeFork { source }).unwrap() else { panic!("fork child") };
    let launch = input(&db, &source.child.as_ref().unwrap().canonical_name);
    let life::Response::ResumeReserved(fork) = super::super::lifecycle_store::handle(&db, &context(&db, Some("pivu")), life::Request::ReserveResume { source, launch }).unwrap() else { panic!("forked") };
    let fork_relations = read(&db, &fork.participant, None, 20).unwrap();
    assert_eq!(fork_relations.upstream.unwrap().name, "pivu");
    let fork_source = fork_relations.fork_source.unwrap();
    assert_eq!(fork_source.name, "mavu");
    assert_eq!(fork_source.incarnation, Some(resumed.participant.incarnation_bits.to_string()));
    assert!(fork_source.stopped_event.is_none());
    assert_eq!(read(&db, &operator, None, 20).unwrap().downstream.len(), 1);

    stop(&db, &resumed.participant);
    let source = resolve(true);
    let event = source.stopped_event.unwrap();
    let life::Response::ResumeFork(source) = super::super::lifecycle_store::handle(&db, &context(&db, None), life::Request::ReserveResumeFork { source }).unwrap() else { panic!("fork child") };
    let launch = input(&db, &source.child.as_ref().unwrap().canonical_name);
    let life::Response::ResumeReserved(fork) = super::super::lifecycle_store::handle(&db, &context(&db, None), life::Request::ReserveResume { source, launch }).unwrap() else { panic!("forked stopped source") };
    let stopped_fork = read(&db, &fork.participant, None, 20).unwrap();
    assert_eq!(stopped_fork.origin, "external");
    assert!(stopped_fork.upstream.is_none());
    let stopped_source = stopped_fork.fork_source.unwrap();
    assert_eq!(stopped_source.name, "mavu");
    assert!(stopped_source.incarnation.is_none());
    assert_eq!(stopped_source.stopped_event, Some(event.to_string()));
}

#[test]
fn launch_relations_unknown_failed_start_and_restart_are_truthful() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agentx.db");
    let db = AgentxDb::open_at(&path).unwrap();
    // A legacy participant can initiate a new exact edge without acquiring a
    // fabricated origin for its own historical creation.
    db.conn().execute("INSERT INTO instances(name,tool,created_at,status) VALUES('lobo','qoder',1,'active')", []).unwrap();
    let a = ParticipantRef { canonical_name: "lobo".into(), incarnation_bits: 1f64.to_bits() };
    let caller = context(&db, Some("lobo"));
    let launch::Response::Reserved(reservation) = super::super::launch_store::handle(&db, &caller, launch::Request::Reserve(input(&db, "mavu"))).unwrap() else { panic!("reserved") };
    assert_eq!(read(&db, &a, None, 20).unwrap().origin, "unknown");
    super::super::launch_store::handle(&db, &caller, launch::Request::CancelFailed(reservation)).unwrap();
    assert_eq!(read(&db, &a, None, 20).unwrap().downstream[0].availability, "unavailable");
    assert_eq!(read(&db, &a, None, 20).unwrap().downstream[0].launch_failure.as_deref(), Some("cancelled_before_ready"));
    let mut unresolved = context(&db, None);
    unresolved.caller.invocation.process_binding_id = Some("expired-binding".into());
    let launch::Response::Reserved(unknown) = super::super::launch_store::handle(&db, &unresolved, launch::Request::Reserve(input(&db, "pivu"))).unwrap() else { panic!("reserved") };
    assert_eq!(read(&db, &unknown.participant, None, 20).unwrap().origin, "unknown");
    let generation = db.database_generation().unwrap();
    drop(db);
    let db = AgentxDb::open_at(&path).unwrap();
    assert_eq!(db.database_generation().unwrap(), generation);
    assert_eq!(read(&db, &a, None, 20).unwrap().downstream[0].name, "mavu");
}

#[test]
fn launch_relations_v25_migration_preserves_existing_state_without_backfill() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agentx.db");
    let db = AgentxDb::open_at(&path).unwrap();
    db.conn().execute_batch("INSERT INTO instances(name,tool,created_at,status) VALUES('lobo','qoder',1,'active'); DROP TABLE agent_creation_instances; DROP TABLE agent_creations; PRAGMA user_version=25;").unwrap();
    drop(db);
    let db = AgentxDb::open_at(&path).unwrap();
    assert!(db.get_instance_full("lobo").unwrap().is_some());
    let legacy = ParticipantRef { canonical_name: "lobo".into(), incarnation_bits: 1f64.to_bits() };
    assert_eq!(read(&db, &legacy, None, 20).unwrap().origin, "unknown");
    assert_eq!(db.conn().query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0)).unwrap(), i64::from(crate::db::SCHEMA_VERSION));
    assert!(!std::fs::read_dir(dir.path()).unwrap().filter_map(Result::ok).any(|entry| entry.file_name().to_string_lossy().contains("archive")), "in-place migration must not archive the database");
}
