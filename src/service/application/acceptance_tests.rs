//! Independent admission evidence: a stale client cannot poison a healthy
//! generation mirror, while durable/mirror divergence must stop all dispatch.

use super::*;
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};

fn context(generation: &str) -> RequestContext {
    RequestContext {
        caller: CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: Some(generation.into()) },
        process: CallerProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
        daemon_instance: "independent-application-fixture".into(),
        database_generation: generation.into(),
    }
}

fn with_application(body: impl FnOnce(&mut SqliteApplication, DatabaseGeneration, String)) {
    let directory = tempfile::Builder::new().prefix("agentx-admission-").tempdir().unwrap();
    let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
    let generation = db.database_generation().unwrap();
    let mirror = DatabaseGeneration::default();
    let mut app = SqliteApplication::new(db, mirror.clone()).unwrap();
    body(&mut app, mirror, generation);
}

fn read() -> Request {
    Request::Events(crate::events::service::Request::Watermark)
}

#[test]
fn agent_names_read_lane_applies_workspace_and_label_scope_without_cleanup() {
    use crate::service::queries::{AgentFilter, AgentWorkspace, Request as Query, Response as View};
    with_application(|app, _, generation| {
        for (name, created, workspace, remote) in [("lobo", 1.0, None, None), ("nava", 2.0, Some("/team-a"), None), ("miri", 3.0, Some("/team-b"), None), ("remote", 4.0, Some("/team-a"), Some("peer"))] {
            app.db
                .save_instance_named(
                    name,
                    serde_json::json!({"created_at":created,"tool":"codex","status":"active","launch_context":serde_json::json!({"collaboration":{"workspace":workspace}}).to_string(),"origin_device_id":remote}).as_object().unwrap(),
                )
                .unwrap();
        }
        let mut lane = app.read_lane().unwrap();
        let workspace = AgentWorkspace::Team("/team-a".into());
        let mut names = |labels| {
            let request = Request::Queries(Query::AgentNames(AgentFilter { workspace: workspace.clone(), labels }));
            assert!((lane.accepts)(&request));
            let Response::Queries(View::AgentNames(names)) = lane.application.handle(&context(&generation), request).unwrap() else { panic!("expected names projection") };
            names
        };
        let watermark = app.db.get_last_event_id();
        app.db
            .with_write_scope(|| {
                assert_eq!(names(vec![]), ["remote", "nava", "lobo"]);
                assert_eq!(names(vec![crate::agents::labels::parse_selector("!topic").unwrap()]), ["lobo", "nava"]);
                Ok(())
            })
            .unwrap();
        assert_eq!(app.db.get_last_event_id(), watermark);
    });
}

#[test]
fn stale_client_rejection_leaves_healthy_generation_available() {
    with_application(|app, mirror, generation| {
        let mut caller = context(&generation);
        caller.caller.expected_database_generation = Some(uuid::Uuid::new_v4().to_string());
        assert!(matches!(app.handle(&caller, read()), Err(ServiceError::StaleGeneration)));
        assert_eq!(mirror.current(), generation);
        assert!(app.handle(&context(&generation), read()).is_ok());
        let mut stale = context(&generation);
        stale.database_generation = uuid::Uuid::new_v4().to_string();
        assert!(matches!(app.handle(&stale, read()), Err(ServiceError::StaleGeneration)));
        assert_eq!(mirror.current(), generation);
    });
}

#[test]
fn divergent_durable_generation_invalidates_mirror_and_stops_dispatch() {
    with_application(|app, mirror, generation| {
        let replacement = uuid::Uuid::new_v4().to_string();
        app.db.kv_set("service:database_generation", Some(&replacement)).unwrap();
        assert!(matches!(app.handle(&context(&generation), read()), Err(ServiceError::Unavailable(_))));
        assert!(mirror.current().is_empty());
        assert!(matches!(app.handle(&context(&replacement), read()), Err(ServiceError::Unavailable(_))));
    });
}

#[test]
fn empty_publication_cannot_be_healed_by_an_ordinary_request() {
    with_application(|app, mirror, generation| {
        mirror.invalidate();
        assert!(matches!(app.handle(&context(&generation), read()), Err(ServiceError::Unavailable(_))));
        assert!(mirror.current().is_empty());
    });
}

#[test]
fn management_reader_serves_committed_state_while_writer_holds_transaction() {
    use crate::management::contract::{Request as Management, Response as View};
    with_application(|app, mirror, generation| {
        let request = Request::Management(Management::HumanMailbox { before: None, pending: true, limit: 20 });
        let mut lane = app.read_lane().unwrap();
        assert!((lane.accepts)(&request));
        assert!(!(lane.accepts)(&Request::Management(Management::HandleHumanMessages { ids: vec![] })));
        assert!(matches!(lane.application.handle(&context(&generation), Request::Management(Management::List { after: None, limit: 20 })), Ok(Response::Management(View::Agents(_)))));
        app.db
            .with_write_scope(|| {
                app.db.insert_event_row("message", "lobo", &serde_json::json!({"scope":"mentions","mentions":["bigboss"]}), None)?;
                assert!(matches!(lane.application.handle(&context(&generation), request.clone()), Ok(Response::Management(View::HumanMailbox(page))) if page.unhandled_count == 0));
                Ok(())
            })
            .unwrap();
        assert!(matches!(lane.application.handle(&context(&generation), request.clone()), Ok(Response::Management(View::HumanMailbox(page))) if page.unhandled_count == 1));
        app.db.kv_set("service:database_generation", Some(&uuid::Uuid::new_v4().to_string())).unwrap();
        assert!(matches!(lane.application.handle(&context(&generation), request), Err(ServiceError::StaleGeneration)));
        assert_eq!(mirror.current(), generation, "only the writer publishes durable generations");
    });
}

#[test]
fn delivery_poll_reader_serves_committed_mail_and_fences_progress() {
    use crate::messaging::{delivery_service as delivery, service as messages};
    use crate::runtime::{hosts, hosts_store};
    use serde_json::json;

    with_application(|app, _, generation| {
        let mut caller = context(&generation);
        caller.caller.invocation.explicit_name = Some("lobo".into());
        app.db.save_instance_named("lobo", json!({"created_at":10.0,"tool":"codex","status":"active","last_event_id":0}).as_object().unwrap()).unwrap();
        let participant = crate::agents::service::resolve_actor(&app.db, &caller).unwrap().participant;
        let child = hosts::ChildProcess { pid: caller.process.pid, incarnation: caller.process.incarnation.clone() };
        let response = app.handle(&caller, Request::Hosts(hosts::Request::RegisterProcess { participant: participant.clone(), child, launch_context: None })).unwrap();
        let Response::Hosts(hosts::Response::ProcessRegistered { registration, .. }) = response else { panic!("expected process registration") };
        let key = "runtime_process:lobo";
        let before = app.db.kv_get(key).unwrap().unwrap();
        let mail = json!({"from":"nova","scope":"mentions","mentions":["lobo"]});
        let committed = app.db.insert_event_row("message", "nova", &mail, None).unwrap();
        let mut lane = app.read_lane().unwrap();
        let poll = Request::Messages(messages::Request::Delivery(delivery::Request::Poll));
        let heartbeat = Request::Messages(messages::Request::Delivery(delivery::Request::Heartbeat { registration }));
        assert!((lane.accepts)(&poll));
        assert!(!(lane.accepts)(&heartbeat));
        let mut read_mail = |context: &RequestContext| {
            let response = lane.application.handle(context, poll.clone()).unwrap();
            let Response::Messages(messages::Response::Delivery(delivery::Response::Snapshot(view))) = response else { panic!("expected delivery snapshot") };
            assert_eq!(view.actor.participant, participant);
            assert_eq!(view.actor.record.last_event_id, 0);
            view.pending.iter().map(|message| message.event_id.unwrap()).collect::<Vec<_>>()
        };
        let mut impostor = caller.clone();
        impostor.process.incarnation = Some("different-process".into());
        assert_eq!(read_mail(&impostor), [committed]);
        let progress = |db: &AgentxDb| hosts_store::delivery_progress(db, &participant).unwrap().unwrap().last_poll;
        assert_eq!(progress(&app.db), None);
        let later = app
            .db
            .with_write_scope(|| {
                let later = app.db.insert_event_row("message", "nova", &mail, None)?;
                assert_eq!(read_mail(&caller), [committed]);
                assert!(progress(&app.db).is_some());
                assert_eq!(app.db.kv_get(key)?.as_deref(), Some(before.as_str()));
                Ok(later)
            })
            .unwrap();
        assert_eq!(read_mail(&caller), [committed, later]);
        assert_eq!(app.db.kv_get(key).unwrap().as_deref(), Some(before.as_str()));
        let last_poll = progress(&app.db).unwrap();
        app.handle(&caller, heartbeat.clone()).unwrap();
        let persisted: serde_json::Value = serde_json::from_str(&app.db.kv_get(key).unwrap().unwrap()).unwrap();
        assert_eq!(persisted["last_poll"], last_poll);
        assert!(persisted.get("protocol").is_none());
        assert_eq!(app.db.get_instance_full("lobo").unwrap().unwrap().last_event_id, 0);

        // Replace the durable registration while retaining participant and peer.
        let mut successor: serde_json::Value = serde_json::from_str(&before).unwrap();
        successor["registration"]["token"] = json!(uuid::Uuid::new_v4().to_string());
        app.db.kv_set(key, Some(&successor.to_string())).unwrap();
        assert_eq!(progress(&app.db), None);
        assert!(matches!(app.handle(&caller, heartbeat), Err(ServiceError::StaleFence)));
    });
}

#[test]
fn nested_query_does_not_disable_the_remaining_read_budget() {
    with_application(|app, _, _| {
        let result = app.db.with_read_budget(
            || true,
            || {
                app.db.with_read_budget(|| false, || ()).unwrap();
                // A projection that falls back must still fail the outer request
                // when the actual SQLite instruction budget was interrupted.
                let _ = app.db.conn().query_row("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<10000) SELECT sum(x) FROM n", [], |row| row.get::<_, i64>(0));
            },
        );
        assert_eq!(result.unwrap_err().sqlite_error_code(), Some(rusqlite::ErrorCode::OperationInterrupted));
        assert_eq!(app.db.conn().query_row("SELECT 1", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
    });
}

#[test]
fn agent_info_read_lane_keeps_stopped_facts_and_bindings_with_the_exact_instance() {
    use crate::service::queries::{
        Request as Query, Response as View,
        info::{Projection, Section, Target},
    };
    use serde_json::json;

    with_application(|app, _, generation| {
        let scope = app.db.path().parent().unwrap().to_owned();
        let old = 10.125f64;
        let current = 11.25f64;
        let profile = json!({"name":"actual","definition":{"tool":"codex","args":["--saved"]},"overrides":["--override"],"replace_args":false,"selected_by":"explicit_profile"});
        let collaboration = json!({"workspace":scope,"role":{"name":"worker","path":"/fixture/worker.md","namespace":"test","profile":"role-default","prompt":"saved role text","description":null,"default":true},"endpoint":false});
        app.db
            .save_instance_named(
                "lobo",
                json!({"created_at":old,"tool":"codex","directory":scope,"launch_profile":profile.to_string(),"launch_context":json!({"title":"Earlier launch","collaboration":collaboration}).to_string()}).as_object().unwrap(),
            )
            .unwrap();
        let participant = crate::agents::attachments::ParticipantRef { canonical_name: "lobo".into(), incarnation_bits: old.to_bits() };
        crate::agents::launch_relations::record(&app.db, &context(&generation), &participant, false).unwrap();
        let mut actor = context(&generation);
        actor.caller.invocation.explicit_name = Some("lobo".into());
        app.handle(
            &actor,
            Request::Participants(crate::agents::service::Request::Lifecycle(crate::agents::lifecycle_service::Request::Context(crate::agents::context_service::Request::FreezeInstructions {
                participant: participant.clone(),
                instructions: crate::runtime::agent_context::FrozenInstructions { text: "frozen bootstrap".into() },
            }))),
        )
        .unwrap();
        let snapshot = app.db.get_instance_snapshot("lobo").unwrap().unwrap();
        let event = app.db.log_event("life", "lobo", &json!({"action":"stopped","snapshot":snapshot})).unwrap();
        app.db.delete_instance("lobo").unwrap();
        app.db.save_instance_named("lobo", json!({"created_at":current,"tool":"claude","directory":scope}).as_object().unwrap()).unwrap();

        let dir = scope.join(".controller");
        std::fs::create_dir(&dir).unwrap();
        let assignment = |bits| json!({"scope":scope,"generation":generation,"participant":{"canonical_name":"lobo","incarnation_bits":bits}});
        for (name, bits) in [("old", old.to_bits()), ("new", current.to_bits())] {
            std::fs::write(dir.join(format!("{name}.md")), format!("---\nschema: task\nstatus: active\nagents: [{}]\n---\n{name} body\n", assignment(bits))).unwrap();
        }

        let mut lane = app.read_lane().unwrap();
        let mut read = |target, section, generation| {
            let request = Request::Queries(Query::AgentInfo { target, section });
            assert!((lane.accepts)(&request));
            lane.application.handle(generation, request)
        };

        let caller = context(&generation);
        let stopped = Target::Stopped { name: "lobo".into(), event, bits: Some(old.to_bits()) };
        let same_old_incarnation = Target::Incarnation { name: "lobo".into(), bits: old.to_bits() };
        let successor = Target::Incarnation { name: "lobo".into(), bits: current.to_bits() };
        for target in [stopped.clone(), same_old_incarnation] {
            let Response::Queries(View::AgentInfo(Projection::Facts(facts))) = read(target.clone(), Section::Facts, &caller).unwrap() else { panic!("expected facts") };
            assert_eq!(facts.title.as_deref(), Some("Earlier launch"));
            assert_eq!(facts.status.as_deref(), Some("stopped"));
            assert_eq!(facts.created_at, Some(old));
            assert_eq!(facts.profile.unwrap().name, "actual");
            assert!(facts.role.unwrap().default);
            assert!(matches!(read(target.clone(), Section::Bootstrap, &caller), Ok(Response::Queries(View::AgentInfo(Projection::Text(Some(text))))) if text == "frozen bootstrap"));
            assert!(matches!(read(target, Section::Instructions, &caller), Ok(Response::Queries(View::AgentInfo(Projection::Text(Some(text))))) if text == "saved role text"));
        }
        assert!(
            matches!(read(stopped.clone(), Section::ProfileArguments, &caller), Ok(Response::Queries(View::AgentInfo(Projection::ProfileArguments(Some(profile))))) if profile.definition.args == ["--saved"] && profile.overrides == ["--override"])
        );
        assert!(matches!(read(stopped.clone(), Section::Bindings, &caller), Ok(Response::Queries(View::AgentInfo(Projection::Bindings(files)))) if files.len() == 1 && std::path::Path::new(&files[0].path).ends_with("old.md")));
        assert!(matches!(read(successor.clone(), Section::Bindings, &caller), Ok(Response::Queries(View::AgentInfo(Projection::Bindings(files)))) if files.len() == 1 && std::path::Path::new(&files[0].path).ends_with("new.md")));
        assert!(matches!(read(successor, Section::Bootstrap, &caller), Ok(Response::Queries(View::AgentInfo(Projection::Text(None))))));
        let forged = Target::Stopped { name: "lobo".into(), event, bits: Some(current.to_bits()) };
        assert!(matches!(read(forged, Section::Facts, &caller), Err(ServiceError::StaleFence)));
        assert!(matches!(read(stopped, Section::Facts, &context("stale-generation")), Err(ServiceError::StaleGeneration)));
    });
}
