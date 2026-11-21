//! Independent daemon-side host fence cases using real isolated persistence.
//! RequestContext here models transport-observed evidence, never caller JSON.

use super::*;
use crate::agents::attachments::{AttachmentIntent, AttachmentOwnerId, AttachmentOwnerProcess, AttachmentRequest, AttachmentStore, ExecutionId, ProviderSessionKey};
use crate::runtime::hosts::{ChildProcess, EndpointKind, LaunchWaitRegistration, ProcessRegistration};
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};

fn context() -> RequestContext {
    let pid = std::process::id();
    RequestContext {
        caller: CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: InvocationContext { explicit_name: Some("lobo".into()), ..Default::default() }, deadline_unix_ms: None, expected_database_generation: None },
        process: CallerProcess { pid, incarnation: crate::sys::process::identity(pid) },
        daemon_instance: "independent-host-fixture".into(),
        database_generation: "fixture-generation".into(),
    }
}

fn with_db(body: impl FnOnce(&AgentxDb, RequestContext)) {
    let directory = tempfile::Builder::new().prefix("agentx-host-fence-").tempdir().unwrap();
    let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
    db.save_instance_named("lobo", serde_json::json!({"created_at":10.0,"tool":"codex","status":"active","last_event_id":0}).as_object().unwrap()).unwrap();
    body(&db, context());
}

fn participant(db: &AgentxDb) -> crate::agents::attachments::ParticipantRef {
    SqliteAttachmentStore::new(db).participant("lobo").unwrap()
}

fn process_registration(db: &AgentxDb, context: &RequestContext) -> ProcessRegistration {
    let response = handle(db, context, Request::RegisterProcess { participant: participant(db), child: ChildProcess { pid: context.process.pid, incarnation: context.process.incarnation.clone() }, launch_context: None }).unwrap();
    let Response::ProcessRegistered { registration, .. } = response else {
        panic!("process registration response");
    };
    registration
}

#[test]
fn stolen_visible_attachment_owner_cannot_register_or_renew_as_another_peer() {
    with_db(|db, legitimate| {
        let mut attachments = SqliteAttachmentStore::for_owner_process(db, AttachmentOwnerProcess { pid: legitimate.process.pid, process_identity: legitimate.process.incarnation.clone() });
        let owner = AttachmentOwnerId("visible-controller-token".into());
        let attachment = attachments
            .attach(AttachmentRequest {
                execution: ExecutionId("native-generation".into()),
                owner: owner.clone(),
                session: ProviderSessionKey { provider: "codex".into(), scope: "/fixture/native".into(), session_id: "root-a".into() },
                intent: AttachmentIntent::Create { provisional: participant(db) },
            })
            .unwrap();
        let wake = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = wake.local_addr().unwrap().port();
        let request = Request::RegisterNativeRoot { target: attachment.target.clone(), owner, wake_port: port, state: NativeRootState::Idle };
        let mut impostor = legitimate.clone();
        impostor.process.incarnation = Some("different-peer-incarnation".into());
        assert!(matches!(handle(db, &impostor, request.clone()), Err(ServiceError::StaleFence)));
        assert_eq!(db.get_notify_endpoint_port("lobo", "hook"), None);
        let Response::Registered(registration) = handle(db, &legitimate, request).unwrap() else {
            panic!("host registration response");
        };
        assert!(matches!(handle(db, &impostor, Request::RenewNativeRoot(registration.clone())), Err(ServiceError::StaleFence)));
        assert!(matches!(handle(db, &impostor, Request::ReleaseNativeRoot(registration.clone())), Err(ServiceError::StaleFence)));
        assert_eq!(db.get_notify_endpoint_port("lobo", "hook"), Some(port));
        assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().pid, Some(i64::from(legitimate.process.pid)));
        handle(db, &legitimate, Request::ReleaseNativeRoot(registration)).unwrap();
        assert_eq!(db.get_notify_endpoint_port("lobo", "hook"), None);
    });
}

#[test]
fn process_registration_rejects_stale_participant_child_and_peer_then_fences_ports() {
    with_db(|db, legitimate| {
        let mut stale = participant(db);
        stale.incarnation_bits += 1;
        let child = ChildProcess { pid: legitimate.process.pid, incarnation: legitimate.process.incarnation.clone() };
        assert!(matches!(handle(db, &legitimate, Request::RegisterProcess { participant: stale, child: child.clone(), launch_context: None }), Err(ServiceError::StaleFence)));
        let mut wrong_child = child;
        wrong_child.incarnation = Some("reused-child-pid".into());
        assert!(matches!(handle(db, &legitimate, Request::RegisterProcess { participant: participant(db), child: wrong_child, launch_context: None }), Err(ServiceError::StaleFence)));
        let registration = process_registration(db, &legitimate);
        let wake = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let newer = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let old_port = wake.local_addr().unwrap().port();
        let new_port = newer.local_addr().unwrap().port();
        handle(db, &legitimate, Request::SetProcessEndpoint { registration: registration.clone(), kind: EndpointKind::Injection, port: old_port }).unwrap();
        handle(db, &legitimate, Request::SetProcessEndpoint { registration: registration.clone(), kind: EndpointKind::Injection, port: new_port }).unwrap();
        handle(db, &legitimate, Request::ReleaseProcessEndpoint { registration: registration.clone(), kind: EndpointKind::Injection, port: old_port }).unwrap();
        assert_eq!(db.get_notify_endpoint_port("lobo", "inject"), Some(new_port));
        let mut impostor = legitimate.clone();
        impostor.process.incarnation = Some("other-peer".into());
        assert!(matches!(handle(db, &impostor, Request::ReleaseProcessEndpoint { registration: registration.clone(), kind: EndpointKind::Injection, port: new_port }), Err(ServiceError::StaleFence)));
        assert_eq!(db.get_notify_endpoint_port("lobo", "inject"), Some(new_port));
        handle(db, &legitimate, Request::ReleaseProcessEndpoint { registration, kind: EndpointKind::Injection, port: new_port }).unwrap();
        assert_eq!(db.get_notify_endpoint_port("lobo", "inject"), None);
    });
}

#[test]
fn late_process_registration_cannot_replace_issued_kill_identity() {
    use crate::agents::lifecycle_service::{Request as LifecycleRequest, Response as LifecycleResponse};
    use crate::agents::service::{Request as ParticipantRequest, Response as ParticipantResponse};

    with_db(|db, context| {
        // A saved runner is already tracked; the late Provider is the real,
        // live test process, so it passes ordinary child identity checks.
        let runner_pid = if context.process.pid == 4242 { 4243 } else { 4242 };
        db.update_instance_pid_with_identity("lobo", runner_pid, Some("runner-start-identity")).unwrap();
        let participant = participant(db);
        let lifecycle = |request| match crate::agents::service::handle(db, &context, ParticipantRequest::Lifecycle(request)).unwrap() {
            ParticipantResponse::Lifecycle(response) => response,
            _ => panic!("native lifecycle response required"),
        };
        let LifecycleResponse::KillPrepared(plan) = lifecycle(LifecycleRequest::PrepareKill { participant: participant.clone(), initiator: "operator".into() }) else {
            panic!("kill plan required");
        };
        assert_eq!(plan.ticket.pid, runner_pid);
        assert_eq!(plan.ticket.process_identity.as_deref(), Some("runner-start-identity"));
        assert!(matches!(lifecycle(LifecycleRequest::BeginKill { ticket: plan.ticket.clone() }), LifecycleResponse::KillGranted));
        let before = db.get_instance_full("lobo").unwrap().unwrap();
        let identity = db.get_instance_pid_identity("lobo").unwrap();
        let event_frontier = db.get_last_event_id();
        let event_count: i64 = db.conn().query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0)).unwrap();
        assert_eq!(before.status_context, "exit:killed");
        assert_eq!(before.status_detail, "operator");
        assert!(context.process.incarnation.is_some());
        let late = Request::RegisterProcess { participant, child: ChildProcess { pid: context.process.pid, incarnation: context.process.incarnation.clone() }, launch_context: Some(serde_json::json!({"late_registration":true}).to_string()) };
        assert!(matches!(handle(db, &context, late), Err(ServiceError::StaleFence)));
        let after = db.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!(after.pid, before.pid);
        assert_eq!(db.get_instance_pid_identity("lobo").unwrap(), identity);
        assert_eq!((after.status, after.status_context, after.status_detail, after.status_time), (before.status, before.status_context, before.status_detail, before.status_time));
        assert_eq!(after.launch_context, before.launch_context);
        assert_eq!(db.get_last_event_id(), event_frontier);
        assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM events", [], |row| row.get::<_, i64>(0)).unwrap(), event_count);

        let LifecycleResponse::KillCompleted(first) = lifecycle(LifecycleRequest::CompleteKill { ticket: plan.ticket.clone() }) else {
            panic!("kill completion required");
        };
        assert!(first.stopped && !first.retained);
        assert!(db.get_instance_full("lobo").unwrap().is_none());
        let completed_frontier = db.get_last_event_id();
        assert!(completed_frontier > event_frontier);
        let completed_count: i64 = db.conn().query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0)).unwrap();
        let LifecycleResponse::KillCompleted(replay) = lifecycle(LifecycleRequest::CompleteKill { ticket: plan.ticket }) else {
            panic!("replayed kill completion required");
        };
        assert_eq!(serde_json::to_value(first).unwrap(), serde_json::to_value(replay).unwrap());
        assert_eq!(db.get_last_event_id(), completed_frontier);
        assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM events", [], |row| row.get::<_, i64>(0)).unwrap(), completed_count);
    });
}

#[test]
fn approval_edges_preserve_detail_and_never_clear_newer_hook_state() {
    with_db(|db, context| {
        let participant = participant(db);
        db.set_status("lobo", "active", "hook:tool-start").unwrap();
        db.update_instance_fields("lobo", serde_json::json!({"status_detail":"gated command detail"}).as_object().unwrap()).unwrap();
        let before = db.get_last_event_id();
        handle(db, &context, Request::ObserveApproval { participant: participant.clone(), approval: true }).unwrap();
        let blocked = db.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!(blocked.status, "blocked");
        assert_eq!(blocked.status_detail, "gated command detail");
        assert!(db.get_last_event_id() > before);
        let edge = db.get_last_event_id();
        handle(db, &context, Request::ObserveApproval { participant: participant.clone(), approval: true }).unwrap();
        assert_eq!(db.get_last_event_id(), edge, "same approval observation duplicated status event");
        db.set_status("lobo", "listening", "hook:turn-end").unwrap();
        db.update_instance_fields("lobo", serde_json::json!({"status_detail":"newer hook detail"}).as_object().unwrap()).unwrap();
        handle(db, &context, Request::ObserveApproval { participant, approval: false }).unwrap();
        let newer = db.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!(newer.status, "listening");
        assert_eq!(newer.status_context, "hook:turn-end");
        assert_eq!(newer.status_detail, "newer hook detail");
        assert_eq!(db.get_last_event_id(), edge, "falling approval edge overwrote newer hook state");
    });
}

#[test]
fn launch_waiter_same_peer_has_unique_tokens_and_release_fences_peer_and_port() {
    with_db(|db, legitimate| {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let register = || {
            let Response::LaunchWaiter(registration) = handle(db, &legitimate, Request::RegisterLaunchWaiter { batch_prefix: Some("batch-a".into()), port }).unwrap() else {
                panic!("launch waiter response");
            };
            registration
        };
        let first = register();
        let second = register();
        assert_ne!(first.name, second.name);
        let mut impostor = legitimate.clone();
        impostor.process.incarnation = Some("other-peer".into());
        assert!(matches!(handle(db, &impostor, Request::ReleaseLaunchWaiter(first.clone())), Err(ServiceError::StaleFence)));
        handle(db, &legitimate, Request::ReleaseLaunchWaiter(LaunchWaitRegistration { name: first.name.clone(), port: if port == u16::MAX { port - 1 } else { port + 1 } })).unwrap();
        assert_eq!(db.get_notify_endpoint_port(&first.name, "launch_wait"), Some(port));
        handle(db, &legitimate, Request::ReleaseLaunchWaiter(first.clone())).unwrap();
        assert_eq!(db.get_notify_endpoint_port(&first.name, "launch_wait"), None);
        assert_eq!(db.get_notify_endpoint_port(&second.name, "launch_wait"), Some(port));
        handle(db, &legitimate, Request::ReleaseLaunchWaiter(second)).unwrap();
    });
}

#[cfg(unix)]
#[test]
fn takeover_exit_evidence_distinguishes_live_reaped_and_invalid_processes() {
    assert!(!crate::sys::process::has_exited(std::process::id()));
    assert!(!crate::sys::process::has_exited(0));
    assert!(!crate::sys::process::has_exited(u32::MAX), "unrepresentable PID is unknown, not positive exit evidence");
    let mut child = std::process::Command::new("/bin/sh").args(["-c", "exit 0"]).env_clear().stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    assert!(crate::sys::process::has_exited(pid));
}

#[test]
fn exit_registration_survives_deleted_row_but_rejects_successor_and_stolen_token() {
    with_db(|db, legitimate| {
        let registration = process_registration(db, &legitimate);
        validate_process_exit_registration(db, &legitimate, &registration).unwrap();
        assert!(db.delete_instance("lobo").unwrap());
        validate_process_exit_registration(db, &legitimate, &registration).unwrap();
        assert!(validate_process_registration(db, &legitimate, &registration).is_err(), "exit-only registration cannot authorize live mutation");
        let mut impostor = legitimate.clone();
        impostor.process.incarnation = Some("another-host".into());
        assert!(matches!(validate_process_exit_registration(db, &impostor, &registration), Err(ServiceError::StaleFence)));
        let mut stolen = registration.clone();
        stolen.token = "different-token".into();
        assert!(matches!(validate_process_exit_registration(db, &legitimate, &stolen), Err(ServiceError::StaleFence)));
        db.save_instance_named("lobo", serde_json::json!({"created_at":11.0,"tool":"codex","status":"active","pid":legitimate.process.pid}).as_object().unwrap()).unwrap();
        assert!(matches!(validate_process_exit_registration(db, &legitimate, &registration), Err(ServiceError::StaleFence)));
        assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().created_at, 11.0);
    });
}

fn delivery(db: &AgentxDb, context: &RequestContext, request: crate::messaging::delivery_service::Request) -> crate::service::ServiceResult<crate::messaging::delivery_service::Response> {
    match crate::messaging::service::handle(db, context, crate::messaging::service::Request::Delivery(request))? {
        crate::messaging::service::Response::Delivery(response) => Ok(response),
        _ => Err(ServiceError::Internal("unexpected native delivery response".into())),
    }
}

#[test]
fn retained_host_exit_preserves_soft_inactive_omp_and_finalizes_deleted_row_once() {
    use crate::messaging::delivery_service::{Request as DeliveryRequest, Response as DeliveryResponse};
    with_db(|db, context| {
        let registration = process_registration(db, &context);
        db.update_instance_fields("lobo", serde_json::json!({"tool":"omp","status":"inactive","status_context":"exit:session_end"}).as_object().unwrap()).unwrap();
        let watermark = db.get_last_event_id();
        assert!(matches!(delivery(db, &context, DeliveryRequest::Finish { registration: registration.clone(), killed: false, exit_code: Some(0), output: None }), Ok(DeliveryResponse::Changed(false))));
        assert!(db.get_instance_full("lobo").unwrap().is_some());
        assert_eq!(db.get_last_event_id(), watermark);
        db.delete_instance("lobo").unwrap();
        let mut launched = context.clone();
        launched.caller.invocation.environment.insert("AGENTX_LAUNCHED".into(), "1".into());
        let request = DeliveryRequest::FinalizeLaunchExit { registration, never_ready: true, elapsed_secs: 2, exit_code: 1, output: Some("fixture launch failure".into()) };
        assert!(matches!(delivery(db, &launched, request.clone()), Ok(DeliveryResponse::Changed(true))));
        let committed = db.get_last_event_id();
        assert!(committed > watermark);
        assert!(matches!(delivery(db, &launched, request), Ok(DeliveryResponse::Changed(false))));
        assert_eq!(db.get_last_event_id(), committed);
        assert!(db.get_instance_full("lobo").unwrap().is_none());
    });
}

#[test]
fn launch_exit_reports_early_crash_but_suppresses_failure_after_recorded_kill() {
    use crate::messaging::delivery_service::{LaunchObservation, Request as DeliveryRequest, Response as DeliveryResponse};
    for killed in [false, true] {
        with_db(|db, mut context| {
            let registration = process_registration(db, &context);
            context.caller.invocation.environment.insert("AGENTX_LAUNCHED".into(), "1".into());
            context.caller.invocation.environment.insert("AGENTX_LAUNCH_START_EVENT_ID".into(), db.get_last_event_id().to_string());
            delivery(db, &context, DeliveryRequest::ObserveLaunch { registration: registration.clone(), observation: LaunchObservation::Ready { cleared_block: false } }).unwrap();
            if killed {
                db.log_event("life", "lobo", &serde_json::json!({"action":"stopped","reason":"killed"})).unwrap();
            }
            let watermark = db.get_last_event_id();
            let response = delivery(db, &context, DeliveryRequest::FinalizeLaunchExit { registration, never_ready: false, elapsed_secs: 3, exit_code: 1, output: None }).unwrap();
            let DeliveryResponse::Changed(changed) = response else {
                panic!("exit finalization response");
            };
            assert_eq!(changed, !killed);
            assert_eq!(db.get_last_event_id() > watermark, !killed);
        });
    }
}

#[test]
fn claude_mod_poll_tokens_fence_peer_session_successor_and_overlapping_ports() {
    for change in ["peer", "session", "successor", "overlap"] {
        with_db(|db, mut context| {
            db.update_instance_fields("lobo", serde_json::json!({"tool":"claude","session_id":"session-one","status":"listening"}).as_object().unwrap()).unwrap();
            db.set_session_binding("session-one", "lobo").unwrap();
            db.set_process_binding("mod-process", "session-one", "lobo").unwrap();
            context.caller.invocation.process_binding_id = Some("mod-process".into());
            let first = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let first_port = first.local_addr().unwrap().port();
            // A process/session binding alone cannot impersonate the trusted
            // SessionStart or bounded transcript-lineage validation.
            assert!(matches!(handle(db, &context, Request::BeginClaudeModPoll { session: "session-one".into(), port: first_port }).unwrap(), Response::ClaudeModPoll(None)));
            db.mark_claude_session_validated("session-one", "lobo").unwrap();
            let Response::ClaudeModPoll(Some((registration, _))) = handle(db, &context, Request::BeginClaudeModPoll { session: "session-one".into(), port: first_port }).unwrap() else {
                panic!("registered mod poll");
            };
            let second = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let second_port = second.local_addr().unwrap().port();
            match change {
                "peer" => {
                    context.process.incarnation = Some("stolen-visible-poll-token".into());
                }
                "session" => {
                    db.update_instance_fields("lobo", serde_json::json!({"session_id":"session-two"}).as_object().unwrap()).unwrap();
                    db.set_process_binding("mod-process", "session-two", "lobo").unwrap();
                }
                "successor" => {
                    db.update_instance_fields("lobo", serde_json::json!({"created_at":99.0}).as_object().unwrap()).unwrap();
                }
                "overlap" => {
                    let Response::ClaudeModPoll(Some((newer, _))) = handle(db, &context, Request::BeginClaudeModPoll { session: "session-one".into(), port: second_port }).unwrap() else {
                        panic!("overlapping mod poll");
                    };
                    assert_ne!(newer.token, registration.token);
                }
                _ => unreachable!(),
            }
            let result = handle(db, &context, Request::FinishClaudeModPoll { registration, claim: true });
            match change {
                "peer" | "successor" => {
                    assert!(matches!(result, Err(ServiceError::StaleFence)), "{change}");
                    assert_eq!(db.get_notify_endpoint_port("lobo", "plugin"), Some(first_port));
                }
                "session" => {
                    assert!(matches!(result, Ok(Response::ClaudeModWake(false))));
                }
                "overlap" => {
                    result.unwrap();
                    assert_eq!(db.get_notify_endpoint_port("lobo", "plugin"), Some(second_port));
                }
                _ => unreachable!(),
            }
            assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().last_event_id, 0);
        });
    }
}

#[test]
fn message_delivery_poll_uses_bounded_exact_mailbox_window() {
    use crate::messaging::delivery_service::{Request, Response};
    use serde_json::json;

    with_db(|db, context| {
        let registration = process_registration(db, &context);
        let before = db.insert_event_row("message", "nova", &json!({"from":"nova","scope":"mentions","mentions":["lobo"]}), None).unwrap();
        db.conn().execute("UPDATE instances SET last_event_id=?1 WHERE name='lobo'", [before]).unwrap();
        assert!(db.start_mailbox_routes().unwrap());
        while db.backfill_mailbox_routes().unwrap() {}
        let limit = crate::shared::MAX_MESSAGES_PER_DELIVERY;
        let eligible: Vec<_> = (0..limit + 5)
            .map(|_| {
                db.insert_event_row("message", "nova", &json!({"from":"nova","scope":"mentions","mentions":["elsewhere"]}), None).unwrap();
                db.insert_event_row("message", "lobo", &json!({"from":"lobo","scope":"broadcast"}), None).unwrap();
                db.insert_event_row("message", "nova", &json!({"from":"nova","scope":"mentions","mentions":["lobo"]}), None).unwrap()
            })
            .collect();

        let Response::Snapshot(snapshot) = delivery(db, &context, Request::Poll).unwrap() else { panic!("expected a PTY delivery snapshot") };
        assert_eq!(snapshot.actor.participant, registration.participant);
        assert_eq!(snapshot.pending.iter().map(|message| message.event_id.unwrap()).collect::<Vec<_>>(), eligible[..limit]);
        assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().last_event_id, before);
        assert!(delivery_progress(db, &registration.participant).unwrap().unwrap().last_poll.is_some());
    });
}

#[test]
fn codex_completion_cannot_cross_the_current_turn_or_consume_mail() {
    use crate::agents::model_request::RequestPhase;
    use crate::messaging::delivery_service::{GateObservation, Request, Response};
    use serde_json::json;

    with_db(|db, context| {
        let registration = process_registration(db, &context);
        let stamp = crate::shared::time::now_epoch_i64() - 10;
        let model = |turn: &str| json!({"session_id":"session","turn_id":turn,"model":"fixture","phase":"started","last_progress_at":stamp,"error":null,"retired_turns":[]}).to_string();
        let set_turn = |turn: &str, status: &str| {
            db.conn().execute("UPDATE instances SET session_id='session', status=?1, status_context='model:request', status_time=?2, model_request=?3 WHERE name='lobo'", rusqlite::params![status, stamp, model(turn)]).unwrap();
        };
        let request = Request::ObserveGate {
            registration,
            observation: GateObservation::CodexTurnCompleted { session: "session".into(), turn: "first".into(), phase: RequestPhase::Started, progress_at: stamp, status_time: stamp, cursor: 0, completed_at_ms: (stamp + 1) * 1_000 },
        };
        set_turn("first", "active");
        assert!(matches!(delivery(db, &context, request.clone()), Ok(Response::Changed(false))));
        db.insert_event_row("message", "nova", &json!({"from":"nova","scope":"mentions","mentions":["lobo"]}), None).unwrap();
        set_turn("newer", "active");
        assert!(matches!(delivery(db, &context, request.clone()), Ok(Response::Changed(false))));
        set_turn("first", "blocked");
        assert!(matches!(delivery(db, &context, request.clone()), Ok(Response::Changed(false))));
        set_turn("first", "active");
        assert!(matches!(delivery(db, &context, request.clone()), Ok(Response::Changed(true))));
        let row = db.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!((row.status.as_str(), row.status_context.as_str(), row.last_event_id), ("listening", "pty:codex-turn-complete", 0));
        assert_eq!(row.model_request.unwrap().phase, RequestPhase::Started);
        assert!(matches!(delivery(db, &context, request), Ok(Response::Changed(false))));
    });
}

#[test]
fn message_delivery_heartbeat_coalesces_writes_and_repairs_missing_tcp_presence() {
    use crate::messaging::delivery_service::{Request, Response};

    with_db(|db, context| {
        let registration = process_registration(db, &context);
        let heartbeat = Request::Heartbeat { registration };
        assert!(matches!(delivery(db, &context, heartbeat.clone()), Ok(Response::Changed(true))));
        let first = db.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!(first.tcp_mode, 1);
        db.conn().execute_batch("CREATE TEMP TRIGGER reject_repeat BEFORE UPDATE OF last_stop ON instances BEGIN SELECT RAISE(ABORT,'repeated heartbeat wrote'); END").unwrap();
        assert!(matches!(delivery(db, &context, heartbeat.clone()), Ok(Response::Changed(true))));
        db.conn().execute_batch("DROP TRIGGER reject_repeat; UPDATE instances SET last_stop=0 WHERE name='lobo'").unwrap();
        assert!(matches!(delivery(db, &context, heartbeat.clone()), Ok(Response::Changed(true))));
        assert!(db.get_instance_full("lobo").unwrap().unwrap().last_stop > 0);
        db.conn().execute_batch("UPDATE instances SET tcp_mode=0 WHERE name='lobo'").unwrap();
        assert!(matches!(delivery(db, &context, heartbeat), Ok(Response::Changed(true))));
        assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().tcp_mode, 1);
    });
}

#[test]
fn message_delivery_poll_progress_requires_registered_peer_and_preserves_legacy_unknown() {
    with_db(|db, context| {
        let registration = process_registration(db, &context);
        let target = &registration.participant;
        assert!(delivery_progress(db, target).unwrap().unwrap().last_poll.is_none());
        let mut impostor = context.clone();
        impostor.process.incarnation = Some("unrelated-hook".into());
        observe_delivery_poll(db, &impostor, target).unwrap();
        assert!(delivery_progress(db, target).unwrap().unwrap().last_poll.is_none());
        observe_delivery_poll(db, &context, target).unwrap();
        let progress = delivery_progress(db, target).unwrap().unwrap();
        assert_eq!(progress.protocol, Some(crate::daemon::PROTOCOL_VERSION));
        assert!(progress.last_poll.is_some());
        assert_eq!(progress.process_alive, Some(true));
        let mut successor = target.clone();
        successor.incarnation_bits += 1;
        assert!(delivery_progress(db, &successor).unwrap().is_none());
        let key = format!("runtime_process:{}", target.canonical_name);
        let mut legacy: serde_json::Value = serde_json::from_str(&db.kv_get(&key).unwrap().unwrap()).unwrap();
        legacy.as_object_mut().unwrap().remove("last_poll");
        legacy.as_object_mut().unwrap().remove("protocol");
        db.kv_set(&key, Some(&legacy.to_string())).unwrap();
        let legacy = delivery_progress(db, target).unwrap().unwrap();
        assert!(legacy.last_poll.is_none() && legacy.protocol.is_none());
    });
}
