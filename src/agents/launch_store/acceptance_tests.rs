//! Independent native launch reservation, replay and ownership evidence.
use super::*;
use crate::runtime::hosts::ChildProcess;
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};

fn with_db(body: impl FnOnce(&AgentxDb, RequestContext, Reserve)) {
    let directory = tempfile::Builder::new().prefix("agentx-launch-acceptance-").tempdir().unwrap();
    let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
    let pid = std::process::id();
    let context = RequestContext {
        caller: CallerContext { request_id: RequestId("launch-reserve-stable".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid, incarnation: crate::sys::process::identity(pid) },
        daemon_instance: "launch-fixture".into(),
        database_generation: db.database_generation().unwrap(),
    };
    let input = Reserve {
        name: Some("lobo".into()),
        tool: "codex".into(),
        directory: directory.path().to_path_buf(),
        process_binding: "fixture-process-binding".into(),
        prior_session: None,
        resume_cursor: None,
        background: false,
        launch_args: vec!["--model".into(), "fixture-model".into()],
        launch_profile: None,
        collaboration: None,
        title: None,
        labels: Default::default(),
    };
    body(&db, context, input);
}

fn reservation(response: Response) -> Reservation {
    let Response::Reserved(value) = response else {
        panic!("launch reservation response");
    };
    value
}

#[test]
fn lost_reserve_reply_replays_exact_lease_and_changed_intent_or_peer_conflicts() {
    with_db(|db, context, input| {
        db.log_event("life", "fixture", &json!({"action":"watermark"})).unwrap();
        let watermark = db.get_last_event_id();
        let first = reservation(handle(db, &context, Request::Reserve(input.clone())).unwrap());
        assert_eq!(first.launch_cursor, watermark);
        assert_eq!(first.initial_cursor, watermark);
        assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().last_event_id, watermark);
        assert_eq!(serde_json::from_str::<Vec<String>>(&db.get_instance_full("lobo").unwrap().unwrap().launch_args.unwrap()).unwrap(), input.launch_args);
        assert_eq!(db.get_process_binding(&input.process_binding).unwrap().as_deref(), Some("lobo"));
        assert_eq!(reservation(handle(db, &context, Request::Reserve(input.clone())).unwrap()), first);
        assert_eq!(db.conn().query_row::<i64, _, _>("SELECT count(*) FROM instances", [], |row| row.get(0)).unwrap(), 1);
        let mut changed = input.clone();
        changed.launch_args.push("--different-intent".into());
        assert!(matches!(handle(db, &context, Request::Reserve(changed)), Err(ServiceError::Conflict(_))));
        let mut impostor = context.clone();
        impostor.process.incarnation = Some("different-peer".into());
        assert!(handle(db, &impostor, Request::Reserve(input)).is_err());
        assert!(matches!(handle(db, &impostor, Request::CancelFailed(first.clone())), Err(ServiceError::StaleFence)));
        let mut stolen = first.clone();
        stolen.token = "wrong-lease".into();
        assert!(matches!(handle(db, &context, Request::TerminalLaunched { reservation: stolen, requested: None, effective: "fixture".into(), background: None }), Err(ServiceError::StaleFence)));
        db.update_instance_fields("lobo", json!({"created_at":99.0}).as_object().unwrap()).unwrap();
        assert!(matches!(handle(db, &context, Request::CancelFailed(first.clone())), Err(ServiceError::StaleFence)));
        assert!(matches!(handle(db, &context, Request::FinishSupervisor(first)), Err(ServiceError::StaleFence)));
        assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().created_at, 99.0);
    });
}

#[test]
fn reserve_failure_rolls_back_identity_binding_and_retained_acceptance() {
    with_db(|db, context, input| {
        db.conn().execute_batch("CREATE TRIGGER fixture_launch_failure BEFORE INSERT ON process_bindings BEGIN SELECT RAISE(ABORT, 'fixture launch binding failure'); END;").unwrap();
        assert!(handle(db, &context, Request::Reserve(input.clone())).is_err());
        assert!(db.get_instance_full("lobo").unwrap().is_none());
        assert!(db.get_process_binding(&input.process_binding).unwrap().is_none());
        db.conn().execute_batch("DROP TRIGGER fixture_launch_failure").unwrap();
        let first = reservation(handle(db, &context, Request::Reserve(input.clone())).unwrap());
        assert_eq!(reservation(handle(db, &context, Request::Reserve(input)).unwrap()), first);
    });
}

#[test]
fn launch_profile_retains_the_adopted_definition_after_file_changes() {
    with_db(|db, context, mut input| {
        use crate::config::profiles::{Edit, LaunchProfile, ProfileCatalog, edit};
        let scope = db.path().parent().unwrap();
        let definition = LaunchProfile { tool: "codex".into(), description: None, args: input.launch_args.clone() };
        edit(scope, Edit::Create { name: "deep".into(), profile: definition.clone() }).unwrap();
        input.launch_profile = Some(crate::runtime::launch_profile::LaunchProvenance { name: "deep".into(), definition, overrides: vec![], replace_args: false, selected_by: "explicit_profile".into() });
        handle(db, &context, Request::Reserve(input.clone())).unwrap();
        edit(scope, Edit::Update { name: "deep".into(), description: None, args: Some(vec!["--model".into(), "changed".into()]) }).unwrap();
        edit(scope, Edit::Delete { name: "deep".into() }).unwrap();
        assert!(ProfileCatalog::load(scope).unwrap().profiles.is_empty());
        let row = db.get_instance_full("lobo").unwrap().unwrap();
        let retained: crate::runtime::launch_profile::LaunchProvenance = serde_json::from_str(row.launch_profile.as_deref().unwrap()).unwrap();
        assert_eq!(Some(&retained), input.launch_profile.as_ref());
        let snapshot = db.get_instance_snapshot("lobo").unwrap().unwrap();
        assert_eq!(snapshot["launch_profile"].as_str(), row.launch_profile.as_deref());
        let original: Vec<String> = serde_json::from_str(row.launch_args.as_deref().unwrap()).unwrap();
        let (reused, _) = crate::runtime::resume_client::compose_resume_args(crate::integrations::tool::Tool::Codex, &original, &["resume".into(), "session".into()], &[]);
        assert_eq!(reused, input.launch_args);
    });
}

#[test]
fn inactive_local_explicit_name_can_be_consumed_but_live_or_remote_name_cannot() {
    for (status, remote, allowed) in [("inactive", false, true), ("active", false, false), ("inactive", true, false)] {
        with_db(|db, context, input| {
            db.save_instance_named("lobo", json!({"created_at":10.0,"tool":"codex","status":status,"origin_device_id":if remote { "other-device" } else { "" }}).as_object().unwrap()).unwrap();
            let result = handle(db, &context, Request::Reserve(input));
            assert_eq!(result.is_ok(), allowed, "status={status}, remote={remote}");
            if !allowed {
                assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().created_at, 10.0);
            }
        });
    }
}

#[cfg(unix)]
#[test]
fn background_runner_metadata_does_not_replace_registered_pty_child() {
    struct OwnedChild(std::process::Child);
    impl Drop for OwnedChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    with_db(|db, context, input| {
        let reserved = reservation(handle(db, &context, Request::Reserve(input.clone())).unwrap());
        let runner = OwnedChild(std::process::Command::new("/bin/sh").args(["-c", "read fixture_line"]).env_clear().stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap());
        let mut host_context = context.clone();
        host_context.caller.invocation.process_binding_id = Some(input.process_binding);
        crate::runtime::hosts_store::handle(
            db,
            &host_context,
            crate::runtime::hosts::Request::RegisterProcess { participant: reserved.participant.clone(), child: ChildProcess { pid: context.process.pid, incarnation: context.process.incarnation.clone() }, launch_context: None },
        )
        .unwrap();
        handle(
            db,
            &context,
            Request::TerminalLaunched {
                reservation: reserved,
                requested: Some("fixture-terminal".into()),
                effective: "fixture-terminal".into(),
                background: Some(BackgroundProcess { child: ChildProcess { pid: runner.0.id(), incarnation: crate::sys::process::identity(runner.0.id()) }, log_file: "fixture-runner.log".into() }),
            },
        )
        .unwrap();
        let row = db.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!(row.pid, Some(i64::from(context.process.pid)));
        assert_eq!(db.get_instance_pid_identity("lobo").unwrap(), context.process.incarnation);
        assert_eq!(row.background_log_file, "fixture-runner.log");
    });
}

#[test]
fn launch_title_survives_recapture_reopen_and_stopped_api() {
    with_db(|db, mut context, mut input| {
        input.title = Some("  API 负责人  ".into());
        let reserved = reservation(handle(db, &context, Request::Reserve(input.clone())).unwrap());
        let reference = crate::management::contract::AgentRef { name: "lobo".into(), incarnation: Some(reserved.participant.incarnation_bits.to_string()), stopped_event: None };
        assert_eq!(crate::management::store::agent(db, &reference).unwrap().title.as_deref(), Some("API 负责人"));
        context.caller.invocation.launch_context = Some(json!({"title":"ambient overwrite", "terminal_id":"recaptured"}));
        super::super::invocation_binding::capture_launch_context(db, &context, "lobo").unwrap();
        let reopened = AgentxDb::open_at(db.path()).unwrap();
        let row = reopened.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!(crate::agents::launch_title::from_context(row.launch_context.as_deref()).as_deref(), Some("API 负责人"));
        let snapshot = db.get_instance_snapshot("lobo").unwrap().unwrap();
        db.log_event("life", "lobo", &json!({"action":"stopped", "snapshot":snapshot})).unwrap();
        let stopped = crate::management::contract::AgentRef { name: "lobo".into(), incarnation: None, stopped_event: Some(db.get_last_event_id().to_string()) };
        assert_eq!(crate::management::store::agent(db, &stopped).unwrap().title.as_deref(), Some("API 负责人"));
        input.title = Some("Changed intent".into());
        assert!(matches!(handle(db, &context, Request::Reserve(input)), Err(ServiceError::Conflict(_))));
    });
}

#[test]
fn invalid_title_cannot_reserve_and_omitted_title_is_null() {
    with_db(|db, mut context, mut input| {
        input.title = Some("bad\nline".into());
        assert!(matches!(handle(db, &context, Request::Reserve(input.clone())), Err(ServiceError::InvalidInput(_))));
        assert!(db.get_instance_full("lobo").unwrap().is_none());
        context.caller.request_id = RequestId("omitted-title".into());
        input.title = None;
        handle(db, &context, Request::Reserve(input)).unwrap();
        let row = db.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!(crate::agents::launch_title::from_context(row.launch_context.as_deref()), None);
    });
}
