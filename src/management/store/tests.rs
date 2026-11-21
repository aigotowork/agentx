//! Journal state cases for private tickets, exact workers and durable replay.
//! All runtime effects are excluded: only native store requests touch disposable databases.
use super::*;
use crate::service::{CallerContext, InvocationContext, RequestId};
use serde_json::json;

fn with_db(body: impl FnOnce(&AgentxDb, RequestContext)) {
    let root = tempfile::Builder::new().prefix("agentx-management-journal-").tempdir().unwrap();
    let db = AgentxDb::open_at(&root.path().join("agentx.db")).unwrap();
    body(&db, context(&db));
}

fn context(db: &AgentxDb) -> RequestContext {
    let pid = std::process::id();
    let generation = db.database_generation().unwrap();
    RequestContext {
        caller: CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: Some(generation.clone()) },
        process: CallerProcess { pid, incarnation: Some(crate::sys::process::identity(pid).expect("test host needs native OS process identity")) },
        daemon_instance: "journal-test-daemon".into(),
        database_generation: generation,
    }
}

fn launch(prompt: &str) -> Command {
    Command::Launch(LaunchInput { labels: Default::default(), title: None, role: None, workspace: None, tool: Some("codex".into()), profile: None, directory: None, prompt: Some(prompt.into()) })
}

fn submit(db: &AgentxDb, context: &RequestContext, id: &str, command: Command, admit: bool) -> ServiceResult<WorkGrant> {
    match handle(db, context, Request::Submit { request_id: id.into(), command, admit })? {
        Response::Grant(grant) => Ok(grant),
        response => panic!("expected journal grant, got {response:?}"),
    }
}

fn accept(db: &AgentxDb, context: &RequestContext, command: Command) -> (String, String) {
    let id = uuid::Uuid::new_v4().to_string();
    let grant = submit(db, context, &id, command, true).unwrap();
    assert_eq!(grant.operation.status, OperationStatus::Accepted);
    (id, grant.ticket.expect("first admission grants one ticket"))
}

fn claim(db: &AgentxDb, context: &RequestContext, id: &str, ticket: &str) -> ServiceResult<Response> {
    handle(db, context, Request::Claim { request_id: id.into(), ticket: ticket.into() })
}

fn operation(db: &AgentxDb, context: &RequestContext, id: &str) -> Operation {
    let Response::Operation(operation) = handle(db, context, Request::Operation { request_id: id.into() }).unwrap() else {
        panic!("expected operation receipt");
    };
    operation
}

fn finish(db: &AgentxDb, context: &RequestContext, id: &str, ticket: &str, result: Result<CommandResult, String>) -> ServiceResult<Operation> {
    match handle(db, context, Request::Finish { request_id: id.into(), ticket: ticket.into(), result })? {
        Response::Operation(operation) => Ok(operation),
        response => panic!("expected finished receipt, got {response:?}"),
    }
}

fn launched(name: &str) -> CommandResult {
    CommandResult::Launched(LaunchOutcome { agents: vec![name.into()], failed: 0, errors: Vec::new(), agent_refs: None })
}

#[test]
fn native_completeness_launch_receipt_replays_original_incarnation_after_name_reuse() {
    with_db(|db, context| {
        let command = launch("exact launch");
        let (id, ticket) = accept(db, &context, command.clone());
        claim(db, &context, &id, &ticket).unwrap();
        let save_current = |created_at| {
            db.save_instance_named("toku", json!({"created_at": created_at, "tool": "codex"}).as_object().unwrap()).unwrap();
        };
        save_current(10.0);
        let original = AgentRef { name: "toku".into(), incarnation: Some(10.0_f64.to_bits().to_string()), stopped_event: None };
        let result = super::super::worker::launch_result(crate::runtime::launcher::LaunchResult {
            tool: "codex".into(),
            batch_id: "launch-fixture".into(),
            launched: 1,
            failed: 0,
            background: true,
            log_files: vec![],
            handles: vec![json!({"instance_name":"toku"})],
            errors: vec![],
            stdio_exit_code: None,
            attach_participant: None,
            launched_participants: vec![ParticipantRef { canonical_name: "toku".into(), incarnation_bits: 10.0_f64.to_bits() }],
        });
        assert_eq!(agent(db, &original).unwrap().agent, original);
        let mut mismatched = result.clone();
        let CommandResult::Launched(outcome) = &mut mismatched else { unreachable!() };
        outcome.agent_refs.as_mut().unwrap()[0].name = "different".into();
        assert!(matches!(finish(db, &context, &id, &ticket, Ok(mismatched)), Err(ServiceError::InvalidInput(_))));
        let completed = finish(db, &context, &id, &ticket, Ok(result)).unwrap();
        assert_eq!(completed.status, OperationStatus::Succeeded);
        assert!(db.delete_instance("toku").unwrap());
        save_current(20.0);
        let replay = submit(db, &context, &id, command, false).unwrap();
        assert!(replay.ticket.is_none());
        for receipt in [operation(db, &context, &id), replay.operation] {
            let Some(CommandResult::Launched(result)) = receipt.result else { panic!("expected original launch result") };
            assert_eq!(result.agents, ["toku"]);
            assert_eq!(result.agent_refs.as_deref(), Some(std::slice::from_ref(&original)));
        }
        assert!(matches!(agent(db, &original), Err(ServiceError::StaleFence)));
        let successor = AgentRef { incarnation: Some(20.0_f64.to_bits().to_string()), ..original };
        assert_eq!(agent(db, &successor).unwrap().agent, successor);

        // An older worker/journal shape remains readable, even with a namesake.
        let (legacy_id, legacy_ticket) = accept(db, &context, launch("legacy launch"));
        claim(db, &context, &legacy_id, &legacy_ticket).unwrap();
        let legacy = serde_json::from_value(json!({"Launched":{"agents":["toku"],"failed":0,"errors":[]}})).unwrap();
        finish(db, &context, &legacy_id, &legacy_ticket, Ok(legacy)).unwrap();
        let result = serde_json::to_value(operation(db, &context, &legacy_id).result.unwrap()).unwrap();
        assert!(result["Launched"].get("agent_refs").is_none());
    });
}

#[test]
fn identical_replay_never_grants_a_second_ticket_even_when_admission_is_busy() {
    with_db(|db, context| {
        let command = launch("one operation");
        let (id, ticket) = accept(db, &context, command.clone());
        for admit in [true, false] {
            let replay = submit(db, &context, &id, command.clone(), admit).unwrap();
            assert!(replay.ticket.is_none());
            assert_eq!(replay.operation.request_id, id);
            assert_eq!(replay.operation.status, OperationStatus::Accepted);
        }
        assert_eq!(load(db, &context, &id).unwrap().ticket, ticket);
        let new_id = uuid::Uuid::new_v4().to_string();
        assert!(matches!(submit(db, &context, &new_id, command, false), Err(ServiceError::Unavailable(_))));
        assert!(matches!(load(db, &context, &new_id), Err(ServiceError::NotFound(_))));
        assert!(matches!(submit(db, &context, &id, launch("different body"), false), Err(ServiceError::Conflict(_))));
        assert_eq!(load(db, &context, &id).unwrap().command, launch("one operation"));
    });
}

#[test]
fn claim_needs_the_secret_and_os_identity_and_is_granted_only_once() {
    with_db(|db, context| {
        let command = launch("frozen command");
        let (id, ticket) = accept(db, &context, command.clone());
        assert!(matches!(claim(db, &context, &id, "wrong-ticket"), Err(ServiceError::Conflict(_))));
        let mut anonymous = context.clone();
        anonymous.process.incarnation = None;
        assert!(matches!(claim(db, &anonymous, &id, &ticket), Err(ServiceError::Forbidden(_))));
        assert_eq!(operation(db, &context, &id).status, OperationStatus::Accepted);
        let Response::Claimed(frozen) = claim(db, &context, &id, &ticket).unwrap() else {
            panic!("claim must return its accepted command");
        };
        assert_eq!(frozen, command);
        assert_eq!(operation(db, &context, &id).status, OperationStatus::Running);
        assert!(matches!(claim(db, &context, &id, &ticket), Err(ServiceError::Conflict(_))));
        let replay = submit(db, &context, &id, command, false).unwrap();
        assert!(replay.ticket.is_none());
        assert_eq!(replay.operation.status, OperationStatus::Running);
    });
}

#[test]
fn finish_requires_the_claiming_worker_and_keeps_the_first_terminal_result() {
    with_db(|db, context| {
        let (id, ticket) = accept(db, &context, launch("worker bound"));
        assert!(matches!(finish(db, &context, &id, &ticket, Ok(launched("premature"))), Err(ServiceError::Forbidden(_))));
        claim(db, &context, &id, &ticket).unwrap();
        assert!(matches!(finish(db, &context, &id, "wrong-ticket", Ok(launched("wrong"))), Err(ServiceError::Forbidden(_))));
        for different_pid in [true, false] {
            let mut other = context.clone();
            if different_pid {
                other.process.pid = context.process.pid.wrapping_add(1);
            } else {
                other.process.incarnation = Some("different-os-incarnation".into());
            }
            assert!(matches!(finish(db, &other, &id, &ticket, Ok(launched("wrong"))), Err(ServiceError::Forbidden(_))));
        }
        assert_eq!(operation(db, &context, &id).status, OperationStatus::Running);
        let completed = finish(db, &context, &id, &ticket, Ok(launched("toku"))).unwrap();
        assert_eq!(completed.status, OperationStatus::Succeeded);
        let retained = finish(db, &context, &id, &ticket, Ok(launched("replacement"))).unwrap();
        assert_eq!(serde_json::to_value(retained.result).unwrap(), serde_json::to_value(completed.result).unwrap());
        assert_eq!(finish(db, &context, &id, &ticket, Err("late failure".into())).unwrap().status, OperationStatus::Succeeded);
        assert!(matches!(claim(db, &context, &id, &ticket), Err(ServiceError::Conflict(_))));
    });
}

#[test]
fn expired_unclaimed_or_restarted_operations_become_unknown_without_regrant() {
    with_db(|db, context| {
        for expired in [true, false] {
            let command = launch("unclaimed work");
            let (id, ticket) = accept(db, &context, command.clone());
            let mut reader = context.clone();
            if expired {
                let mut record = load(db, &context, &id).unwrap();
                record.accepted_at = crate::shared::time::now_epoch_i64() - 60;
                save(db, &record).unwrap();
            } else {
                reader.daemon_instance = "replacement-daemon".into();
            }
            let stored_key = key(&reader.database_generation, &id);
            let original = db.kv_get(&stored_key).unwrap();
            assert_eq!(operation(db, &reader, &id).status, OperationStatus::Unknown);
            assert_eq!(db.kv_get(&stored_key).unwrap(), original, "receipt observation must leave the journal intact");
            assert!(matches!(claim(db, &reader, &id, &ticket), Err(ServiceError::Conflict(_))));
            let replay = submit(db, &reader, &id, command, false).unwrap();
            assert_eq!(replay.operation.status, OperationStatus::Unknown);
            assert!(replay.ticket.is_none());
            assert!(replay.operation.error.is_some());
            assert_eq!(operation(db, &reader, &id).status, OperationStatus::Unknown);
        }
    });
}

#[test]
fn a_live_worker_may_finish_after_daemon_restart_but_an_os_successor_is_unknown() {
    with_db(|db, context| {
        let (id, ticket) = accept(db, &context, launch("surviving worker"));
        claim(db, &context, &id, &ticket).unwrap();
        let mut restarted = context.clone();
        restarted.daemon_instance = "new-daemon".into();
        assert_eq!(operation(db, &restarted, &id).status, OperationStatus::Running);
        assert_eq!(finish(db, &restarted, &id, &ticket, Ok(launched("toku"))).unwrap().status, OperationStatus::Succeeded);

        let command = launch("dead worker");
        let (id, ticket) = accept(db, &context, command.clone());
        let mut predecessor = context.clone();
        // A positive mismatch on a real live PID deterministically models PID reuse.
        predecessor.process.incarnation = Some("retired-process-incarnation".into());
        claim(db, &predecessor, &id, &ticket).unwrap();
        assert_eq!(operation(db, &context, &id).status, OperationStatus::Unknown);
        let replay = submit(db, &context, &id, command, true).unwrap();
        assert!(replay.ticket.is_none());
        assert_eq!(replay.operation.status, OperationStatus::Unknown);
        assert!(matches!(claim(db, &context, &id, &ticket), Err(ServiceError::Conflict(_))));
        assert!(matches!(finish(db, &context, &id, &ticket, Ok(launched("successor"))), Err(ServiceError::Forbidden(_))));
    });
}

#[test]
fn finish_rejects_a_different_result_type_before_accepting_the_bound_result() {
    with_db(|db, context| {
        let (id, ticket) = accept(db, &context, launch("launch result only"));
        claim(db, &context, &id, &ticket).unwrap();
        let wrong = CommandResult::Stopped(StopOutcome { name: "toku".into(), outcome: "sent".into() });
        assert!(matches!(finish(db, &context, &id, &ticket, Ok(wrong)), Err(ServiceError::InvalidInput(_))));
        assert_eq!(operation(db, &context, &id).status, OperationStatus::Running);
        assert_eq!(finish(db, &context, &id, &ticket, Ok(launched("toku"))).unwrap().status, OperationStatus::Succeeded);
    });
}

#[test]
fn results_are_bound_to_the_exact_agent_or_recipient_of_each_targeted_command() {
    with_db(|db, context| {
        db.save_instance_named("toku", serde_json::json!({"created_at":10.0,"tool":"codex","status":"active","pid":context.process.pid}).as_object().unwrap()).unwrap();
        let agent = AgentRef { name: "toku".into(), incarnation: Some(10.0_f64.to_bits().to_string()), stopped_event: None };
        let message = MessageInput { attachments: vec![], text: "one recipient".into(), intent: Some("inform".into()), reply_to: None, thread: None };
        let other = AgentRef { incarnation: Some(11.0_f64.to_bits().to_string()), ..agent.clone() };
        let cases = [
            (
                Command::Send { agent: agent.clone(), message },
                CommandResult::Sent(SendOutcome { message_id: "1".into(), recipients: vec!["other".into()] }),
                CommandResult::Sent(SendOutcome { message_id: "1".into(), recipients: vec!["toku".into()] }),
            ),
            (Command::Stop(agent.clone()), CommandResult::Stopped(StopOutcome { name: "other".into(), outcome: "sent".into() }), CommandResult::Stopped(StopOutcome { name: "toku".into(), outcome: "sent".into() })),
            (
                Command::Transcript(agent.clone()),
                CommandResult::Transcript(TranscriptPage { agent: other, available: false, reason: None, exchanges: Vec::new() }),
                CommandResult::Transcript(TranscriptPage { agent, available: false, reason: None, exchanges: Vec::new() }),
            ),
        ];
        for (command, wrong, right) in cases {
            let (id, ticket) = accept(db, &context, command);
            claim(db, &context, &id, &ticket).unwrap();
            assert!(matches!(finish(db, &context, &id, &ticket, Ok(wrong)), Err(ServiceError::InvalidInput(_))));
            assert_eq!(operation(db, &context, &id).status, OperationStatus::Running);
            assert_eq!(finish(db, &context, &id, &ticket, Ok(right)).unwrap().status, OperationStatus::Succeeded);
        }
    });
}

#[test]
fn uncertain_finish_is_reconcilable_only_by_the_original_worker() {
    with_db(|db, context| {
        let (id, ticket) = accept(db, &context, launch("uncertain issuance"));
        claim(db, &context, &id, &ticket).unwrap();
        let receipt = finish(db, &context, &id, &ticket, Err("outcome_unknown: host reply lost".into())).unwrap();
        assert_eq!(receipt.status, OperationStatus::Unknown);
        assert!(matches!(claim(db, &context, &id, &ticket), Err(ServiceError::Conflict(_))));
        assert_eq!(finish(db, &context, &id, &ticket, Ok(launched("toku"))).unwrap().status, OperationStatus::Succeeded);
    });
}

#[test]
fn completed_receipt_survives_database_reopen_and_daemon_replacement() {
    let root = tempfile::Builder::new().prefix("agentx-management-reopen-").tempdir().unwrap();
    let path = root.path().join("agentx.db");
    let db = AgentxDb::open_at(&path).unwrap();
    let context = context(&db);
    let command = launch("durable completed operation");
    let (id, ticket) = accept(&db, &context, command.clone());
    claim(&db, &context, &id, &ticket).unwrap();
    let completed = finish(&db, &context, &id, &ticket, Ok(launched("toku"))).unwrap();
    drop(db);
    let db = AgentxDb::open_at(&path).unwrap();
    let mut restarted = context;
    restarted.daemon_instance = "reopened-daemon".into();
    assert_eq!(db.database_generation().unwrap(), restarted.database_generation);
    let replay = submit(&db, &restarted, &id, command, false).unwrap();
    assert!(replay.ticket.is_none());
    assert_eq!(serde_json::to_value(replay.operation).unwrap(), serde_json::to_value(completed).unwrap());
    let mut replaced = restarted;
    replaced.database_generation = "different-dataset-generation".into();
    assert!(matches!(handle(&db, &replaced, Request::Operation { request_id: id }), Err(ServiceError::NotFound(_))));
}

#[test]
fn local_null_and_empty_origins_are_listed_and_selectable_across_pages() {
    with_db(|db, context| {
        // Remote names bracket and interleave local rows so filtering must precede LIMIT.
        let origins = [("aaaa", Some("remote-device")), ("bbbb", Some("")), ("cccc", None), ("dddd", Some("remote-device")), ("eeee", Some("")), ("ffff", Some("remote-device"))];
        for (name, origin) in origins {
            db.save_instance_named(name, serde_json::json!({"created_at":10.0,"tool":"codex","status":"active","origin_device_id":origin}).as_object().unwrap()).unwrap();
        }
        let page = |after, limit| {
            let Response::Agents(page) = handle(db, &context, Request::List { after, limit }).unwrap() else {
                panic!("expected Agent page");
            };
            assert_eq!(page.generation, context.database_generation);
            page
        };
        let all = page(None, MAX_PAGE);
        let expected = vec!["bbbb", "cccc", "eeee"];
        assert_eq!(all.agents.iter().map(|view| view.agent.name.as_str()).collect::<Vec<_>>(), expected);
        assert!(all.next.is_none());
        for view in &all.agents {
            let Response::Agent(selected) = handle(db, &context, Request::Agent(view.agent.clone())).unwrap() else {
                panic!("listed local Agent must be selectable");
            };
            assert_eq!(selected.agent, view.agent);
        }
        for name in ["aaaa", "dddd", "ffff"] {
            let remote = AgentRef { name: name.into(), incarnation: Some(10.0_f64.to_bits().to_string()), stopped_event: None };
            assert!(matches!(handle(db, &context, Request::Agent(remote)), Err(ServiceError::StaleFence)));
        }
        // Size one checks every boundary; size two checks the final partial page.
        for limit in [1, 2] {
            let mut after = None;
            let mut names = Vec::new();
            loop {
                let current = page(after.clone(), limit);
                assert!(!current.agents.is_empty(), "remote rows must not produce an empty continuation page");
                assert!(current.agents.len() <= limit as usize);
                names.extend(current.agents.into_iter().map(|view| view.agent.name));
                assert!(names.len() <= expected.len(), "pagination must terminate without duplicates");
                match current.next {
                    Some(next) => {
                        assert_ne!(after.as_ref(), Some(&next), "cursor must advance");
                        after = Some(next);
                    }
                    None => break,
                }
            }
            assert_eq!(names, expected);
        }
    });
}

#[test]
fn native_completeness_workspace_members_page_checks_adoption_before_limit() {
    with_db(|db, context| {
        let team = if cfg!(windows) { r"C:\team" } else { "/team" };
        let other = if cfg!(windows) { r"C:\other" } else { "/other" };
        let adopted = |workspace: Option<&str>| json!({"collaboration":{"workspace":workspace,"role":null,"endpoint":false}});
        let rows = [
            ("a_other", Some(adopted(Some(other)).to_string()), None),
            ("b_legacy", None, None),
            ("c_unknown", Some(json!({"collaboration":{"workspace":null,"role":null,"endpoint":false,"future":true}}).to_string()), None),
            ("d_remote", Some(adopted(Some(team)).to_string()), Some("remote")),
            ("m_member", Some(adopted(Some(team)).to_string()), Some("")),
            ("z_global", Some(adopted(None).to_string()), None),
        ];
        for (name, launch_context, origin) in rows {
            db.save_instance_named(name, json!({"created_at":10.0,"tool":"codex","launch_context":launch_context,"origin_device_id":origin}).as_object().unwrap()).unwrap();
        }
        let stop = |name, launch_context, origin| db.insert_event_row("life", name, &json!({"action":"stopped","snapshot":{"created_at":5.0,"tool":"codex","launch_context":launch_context,"origin_device_id":origin}}), None).unwrap();
        let first_stop = stop("m_member", adopted(Some(team)), None::<&str>);
        let last_stop = stop("n_stopped", json!(adopted(Some(team)).to_string()), Some(""));
        stop("not_local", adopted(Some(team)), Some("remote"));
        stop("other_stopped", adopted(Some(other)), None);
        stop("legacy_stopped", json!({}), None);

        // The route uses the daemon-owned WAL reader, where the predicate must
        // be registered as well as on the ordinary database connection.
        let reader = AgentxDb::open_reader(db.path()).unwrap();
        let request = |workspace: &str, current_only, after| Request::ListWorkspaceMembers { workspace: workspace.into(), current_only, after, limit: 1 };
        let page = |workspace, current_only, after| {
            let Response::Agents(page) = handle(&reader, &context, request(workspace, current_only, after)).unwrap() else { panic!("expected workspace Agent page") };
            assert_eq!(page.generation, context.database_generation);
            page
        };
        let first = page(team, false, None);
        assert_eq!(first.agents[0].agent.name, "m_member");
        assert!(first.agents[0].agent.incarnation.is_some());
        assert!(first.current_complete);
        let cursor = first.next.expect("stopped members follow current");
        for (workspace, current_only, after) in [(other, false, cursor.clone()), (team, true, cursor.clone()), (team, false, "0:m_member".into())] {
            assert!(matches!(handle(&reader, &context, request(workspace, current_only, Some(after))), Err(ServiceError::InvalidInput(_))));
        }
        let mut replacement = context.clone();
        replacement.database_generation = "different-generation".into();
        assert!(matches!(handle(&reader, &replacement, request(team, false, Some(cursor.clone()))), Err(ServiceError::InvalidInput(_))));
        assert!(matches!(handle(&reader, &context, Request::List { after: Some(cursor.clone()), limit: 1 }), Err(ServiceError::InvalidInput(_))));

        let second = page(team, false, Some(cursor));
        assert_eq!(second.agents[0].agent.stopped_event, Some(last_stop.to_string()));
        let third = page(team, false, second.next);
        assert_eq!(third.agents[0].agent.stopped_event, Some(first_stop.to_string()));
        assert!(third.next.is_none());
        assert!(page(team, true, None).next.is_none());
        let global = page("global", false, None);
        assert_eq!(global.agents[0].agent.name, "z_global");
        assert!(global.next.is_none());
    });
}

#[test]
fn directory_send_replay_keeps_exact_recipient_and_rejects_mismatched_completion() {
    with_db(|db, context| {
        let command = Command::DirectorySend { directory: "/missing-but-frozen-intent".into(), message: MessageInput { attachments: vec![], text: "contact office".into(), intent: Some("request".into()), reply_to: None, thread: None } };
        let (id, ticket) = accept(db, &context, command.clone());
        claim(db, &context, &id, &ticket).unwrap();
        let recipient = AgentRef { name: "mora".into(), incarnation: Some(1.0f64.to_bits().to_string()), stopped_event: None };
        let mut outcome = DirectorySendOutcome {
            entry: DirectoryEntry {
                generation: context.database_generation.clone(),
                directory: "/canonical-target".into(),
                requested_workspace: Some("/child".into()),
                resolved_workspace: Some("/parent".into()),
                role: RoleView { name: "controller".into(), path: "/parent/.agentx/roles/controller.toml".into(), namespace: "local:/parent/.agentx/roles".into(), profile: "fixture".into(), description: None, default: true },
                endpoint: EndpointView { status: "bound".into(), participant: Some(recipient.clone()), candidates: 1, reason: None },
            },
            recipient: recipient.clone(),
            sent: SendOutcome { message_id: "42".into(), recipients: vec!["wrong".into()] },
            created: true,
        };
        assert!(matches!(finish(db, &context, &id, &ticket, Ok(CommandResult::DirectorySent(outcome.clone()))), Err(ServiceError::InvalidInput(_))));
        assert_eq!(operation(db, &context, &id).status, OperationStatus::Running);
        outcome.sent.recipients = vec![recipient.name];
        let expected = serde_json::to_value(&outcome).unwrap();
        assert_eq!(finish(db, &context, &id, &ticket, Ok(CommandResult::DirectorySent(outcome))).unwrap().status, OperationStatus::Succeeded);
        let replay = submit(db, &context, &id, command.clone(), false).unwrap();
        assert!(replay.ticket.is_none());
        let Some(CommandResult::DirectorySent(retained)) = replay.operation.result else { panic!("missing exact routed receipt") };
        assert_eq!(serde_json::to_value(retained).unwrap(), expected);
        let Command::DirectorySend { message, .. } = command else { unreachable!() };
        assert!(matches!(submit(db, &context, &id, Command::DirectorySend { directory: "/other".into(), message }, false), Err(ServiceError::Conflict(_))));
    });
}

#[test]
fn exact_stopped_collaboration_does_not_follow_namesake_or_edited_role_files() {
    with_db(|db, _context| {
        let adopted = serde_json::json!({"workspace":"/original-team","role":{"name":"reviewer","path":"/deleted/roles/reviewer.toml","namespace":"global:/deleted/roles","profile":"original-profile","prompt":"private instructions","description":"original role","default":false},"endpoint":false});
        let saved = serde_json::json!({"name":"mora","tool":"qoder","created_at":1.0,"directory":"/different-cwd","session_id":"saved-session","launch_context":serde_json::json!({"collaboration":adopted}).to_string()});
        db.log_event("life", "mora", &serde_json::json!({"action":"stopped","snapshot":saved})).unwrap();
        let event = db.get_last_event_id();
        db.save_instance_named(
            "mora",
            serde_json::json!({"tool":"qoder","created_at":2.0,"directory":"/new-cwd","launch_context":serde_json::json!({"collaboration":{"workspace":null,"role":null,"endpoint":false}}).to_string()}).as_object().unwrap(),
        )
        .unwrap();
        let stopped = agent(db, &AgentRef { name: "mora".into(), incarnation: None, stopped_event: Some(event.to_string()) }).unwrap();
        let live = agent(db, &AgentRef { name: "mora".into(), incarnation: Some(2.0f64.to_bits().to_string()), stopped_event: None }).unwrap();
        assert_eq!(stopped.collaboration.status, "adopted");
        assert_eq!(stopped.collaboration.workspace.as_deref(), Some("/original-team"));
        let role = stopped.collaboration.role.unwrap();
        assert_eq!(role.profile, "original-profile");
        assert_eq!(role.path, "/deleted/roles/reviewer.toml");
        assert!(!serde_json::to_string(&role).unwrap().contains("private instructions"));
        assert_eq!(live.collaboration.status, "adopted");
        assert!(live.collaboration.workspace.is_none() && live.collaboration.role.is_none());
        assert_eq!(stopped.directory, "/different-cwd");
    });
}

#[test]
fn office_activity_freezes_exact_addresses_and_human_routing_without_replay() {
    with_db(|db, context| {
        for (name, created) in [("lobo", 10.0_f64), ("mora", 20.0_f64)] {
            db.conn().execute("INSERT INTO instances(name,tool,status,created_at,last_event_id) VALUES(?1,'codex','active',?2,0)", rusqlite::params![name, created]).unwrap();
        }
        let before = super::super::activity::read(db, &context, None, 100).unwrap();
        assert!(before.messages.is_empty());
        let sender = crate::shared::SenderIdentity { kind: crate::shared::SenderKind::Instance, name: "lobo".into(), instance_data: None, session_id: None };
        let direct = crate::messaging::submission::persist_message(db, &sender, "work", None, Some(&["mora".into()])).unwrap();
        crate::messaging::submission::persist_message(db, &sender, "for the human", None, Some(&["bigboss".into()])).unwrap();
        // A raw @bigboss string in explicitly Agent-addressed text is not human routing.
        crate::messaging::submission::persist_message(db, &sender, "quoted @bigboss", None, Some(&["mora".into()])).unwrap();
        let human = crate::shared::SenderIdentity { kind: crate::shared::SenderKind::External, name: "bigboss".into(), instance_data: None, session_id: None };
        crate::messaging::submission::persist_message(db, &human, "browser work", None, Some(&["mora".into()])).unwrap();
        let first = super::super::activity::read(db, &context, Some(before.next_after.clone()), 1).unwrap();
        assert!(first.has_more);
        assert_eq!(first.messages.len(), 1);
        assert_eq!(first.messages[0].id, direct.event_id.to_string());
        assert_eq!(first.messages[0].sender.as_ref().unwrap().incarnation.as_deref(), Some(10.0_f64.to_bits().to_string().as_str()));
        assert_eq!(first.messages[0].recipients[0].incarnation.as_deref(), Some(20.0_f64.to_bits().to_string().as_str()));
        assert_eq!(first.human_messages.len(), 1);
        assert_eq!(first.human_messages[0].text, "for the human");
        // Name reuse never rewrites the accepted event's original recipient identity.
        db.conn().execute("UPDATE instances SET created_at=30 WHERE name='mora'", []).unwrap();
        let replay = super::super::activity::read(db, &context, Some(before.next_after), 100).unwrap();
        assert_eq!(replay.messages[0].recipients[0].incarnation.as_deref(), Some(20.0_f64.to_bits().to_string().as_str()));
        assert!(replay.messages[1].human_recipient);
        assert!(!replay.messages[2].human_recipient);
        assert!(replay.messages[3].human_sender);
        assert!(replay.messages[3].sender.is_none());
        let idle = super::super::activity::read(db, &context, Some(replay.next_after), 100).unwrap();
        assert!(idle.messages.is_empty());
        assert_eq!(idle.human_messages.len(), 1);
    });
}

#[test]
fn office_activity_creation_pagination_legacy_and_pending_cursor() {
    with_db(|db, context| {
        db.conn().execute("INSERT INTO instances(name,tool,status,created_at,last_event_id) VALUES('lobo','codex','active',10,0)", []).unwrap();
        let participant = crate::agents::attachments::ParticipantRef { canonical_name: "lobo".into(), incarnation_bits: 10.0_f64.to_bits() };
        db.log_event("life", "lobo", &json!({"action":"created", "participant":participant})).unwrap();
        db.log_event("message", "legacy", &json!({"from":"legacy", "scope":"mentions", "delivered_to":["lobo"], "text":"legacy"})).unwrap();
        let result = super::super::activity::read(db, &context, Some("0".into()), 100).unwrap();
        assert_eq!(result.created.len(), 1);
        assert!(result.messages.is_empty(), "legacy names cannot become exact flights");
        let reference = AgentRef { name: "lobo".into(), incarnation: Some(participant.incarnation_bits.to_string()), stopped_event: None };
        assert_eq!(projected_agent(db, &context, &reference).unwrap().pending_messages, Some(1));
        db.conn().execute("UPDATE instances SET last_event_id=? WHERE name='lobo'", [db.get_last_event_id()]).unwrap();
        assert_eq!(projected_agent(db, &context, &reference).unwrap().pending_messages, Some(0));
        assert!(super::super::activity::read(db, &context, Some("-1".into()), 100).is_err());
        assert!(super::super::activity::read(db, &context, None, 101).is_err());
        // A failed pending query remains unknown, never a fabricated empty inbox.
        db.conn().execute("ALTER TABLE events RENAME TO unavailable_events", []).unwrap();
        assert_eq!(agent(db, &reference).unwrap().pending_messages, None);
        assert!(matches!(projected_agent(db, &context, &reference), Err(ServiceError::Internal(_))));
    });
}

#[test]
fn human_mailbox_index_preserves_historical_recipients_and_changes() {
    with_db(|db, context| {
        let insert = |data| db.insert_event_row("message", "lobo", &data, None).unwrap();
        let first = insert(json!({"scope":"mentions","mentions":{"historical":"bigboss"}}));
        let second = insert(json!({"scope":"mentions","mentions":["bigboss","bigboss"]}));
        insert(json!({"scope":"broadcast","mentions":["bigboss"]}));
        insert(json!({"scope":"mentions","mentions":["elsewhere"],"text":"@bigboss"}));
        insert(json!({"scope":"mentions","mentions":"invalid historical encoding"}));
        let page = |before| super::super::human::read(db, &context, before, true, 1).unwrap();
        assert!(db.start_human_mailbox_index().unwrap());
        assert!(!db.human_mailbox_covered());
        let expected = page(None);
        assert_eq!(expected.messages[0].id, second.to_string());
        assert_eq!(expected.unhandled_count, 2);
        while db.backfill_human_mailbox().unwrap() {}
        assert!(db.human_mailbox_covered());
        assert_eq!(page(None).messages[0].id, expected.messages[0].id);
        assert_eq!(page(expected.next_before).messages[0].id, first.to_string());

        let later = insert(json!({"scope":"mentions","mentions":"[\"bigboss\"]"}));
        assert_eq!(page(None).messages[0].id, later.to_string());
        assert_eq!(super::super::human::handle(db, &context, vec![later.to_string()]).unwrap().unhandled_count, 2);
        db.conn().execute("UPDATE events SET data=json_object('scope','broadcast') WHERE id=?1", [second]).unwrap();
        assert_eq!(page(None).messages[0].id, first.to_string());
        assert!(
            db.with_write_scope(|| -> anyhow::Result<()> {
                insert(json!({"scope":"mentions","mentions":["bigboss"]}));
                anyhow::bail!("rollback")
            })
            .is_err()
        );
        assert_eq!(page(None).unhandled_count, 1);
        db.conn().execute("DELETE FROM events WHERE id=?1", [first]).unwrap();
        assert_eq!(page(None).unhandled_count, 0);
    });
}

#[test]
fn human_mailbox_explicit_handling_retains_history_and_never_consumes_unseen_or_agent_letters() {
    with_db(|db, context| {
        db.conn().execute("INSERT INTO instances(name,tool,status,created_at,last_event_id) VALUES('lobo','codex','active',10,0)", []).unwrap();
        let sender = crate::shared::SenderIdentity { kind: crate::shared::SenderKind::Instance, name: "lobo".into(), instance_data: None, session_id: None };
        let mut ids = Vec::new();
        for index in 0..23 {
            ids.push(crate::messaging::submission::persist_message(db, &sender, &format!("human letter {index}"), None, Some(&["bigboss".into()])).unwrap().event_id.to_string());
        }
        let external = crate::shared::SenderIdentity { kind: crate::shared::SenderKind::External, name: "bigboss".into(), instance_data: None, session_id: None };
        let direct = crate::messaging::submission::persist_message(db, &external, "quoted @bigboss", None, Some(&["lobo".into()])).unwrap().event_id.to_string();
        let first = super::super::human::read(db, &context, None, true, 20).unwrap();
        assert_eq!(first.unhandled_count, 23);
        assert_eq!(first.messages.len(), 20);
        assert!(first.messages.iter().all(|message| message.handled_at.is_none()));
        assert!(first.messages.iter().all(|message| message.sender.as_ref().is_some_and(|sender| sender.name == "lobo" && sender.incarnation.as_deref() == Some(10.0_f64.to_bits().to_string().as_str()))));
        let older = super::super::human::read(db, &context, first.next_before, true, 20).unwrap();
        assert_eq!(older.messages.len(), 3);
        assert!(older.next_before.is_none());
        // Opening/listing alone never changes handling, including multiple pages.
        assert_eq!(super::super::human::read(db, &context, None, true, 20).unwrap().unhandled_count, 23);
        let selected: Vec<String> = first.messages.iter().map(|message| message.id.clone()).collect();
        let before = db.get_last_event_id();
        let marked = super::super::human::handle(db, &context, selected.clone()).unwrap();
        assert_eq!(marked.unhandled_count, 3);
        assert_eq!(db.get_last_event_id(), before, "no lifecycle/delivery event is invented");
        let timestamp = marked.messages[0].handled_at.clone().unwrap();
        let retry = super::super::human::handle(db, &context, selected).unwrap();
        assert_eq!(retry.messages[0].handled_at.as_deref(), Some(timestamp.as_str()));
        assert_eq!(retry.unhandled_count, 3);
        // Validation is all-or-nothing. A real Agent letter cannot be used as human confirmation.
        assert!(super::super::human::handle(db, &context, vec![ids[0].clone(), direct]).is_err());
        assert_eq!(super::super::human::read(db, &context, None, true, 20).unwrap().unhandled_count, 3);
        let reference = AgentRef { name: "lobo".into(), incarnation: Some(10.0_f64.to_bits().to_string()), stopped_event: None };
        assert_eq!(projected_agent(db, &context, &reference).unwrap().pending_messages, Some(1));
        let newer = crate::messaging::submission::persist_message(db, &sender, "arrived after handling", None, Some(&["bigboss".into()])).unwrap().event_id.to_string();
        let pending = super::super::human::read(db, &context, None, true, 20).unwrap();
        assert_eq!(pending.unhandled_count, 4);
        assert_eq!(pending.messages[0].id, newer);
        assert_eq!(super::super::human::read(db, &context, None, false, 100).unwrap().messages.len(), 24);
        for invalid_ids in [vec![], vec!["0".into()], vec!["01".into()], vec!["999999999".into()]] {
            assert!(super::super::human::handle(db, &context, invalid_ids).is_err());
        }
        let mut stale = context.clone();
        stale.database_generation = "replaced-generation".into();
        assert!(matches!(super::super::human::handle(db, &stale, vec![newer]), Err(ServiceError::StaleFence)));
        let reopened = AgentxDb::open_at(db.path()).unwrap();
        assert_eq!(super::super::human::read(&reopened, &context, None, true, 20).unwrap().unhandled_count, 4);
        assert_eq!(super::super::human::read(&reopened, &context, None, false, 100).unwrap().messages.iter().filter(|message| message.handled_at.is_some()).count(), 20);
    });
}

#[test]
fn exact_conversation_index_preserves_pages_and_bounds_without_scanning_unrelated_messages() {
    with_db(|db, context| {
        let name = "target";
        db.save_instance_named(name, json!({"created_at":20.0,"tool":"codex"}).as_object().unwrap()).unwrap();
        let agent = AgentRef { name: name.into(), incarnation: Some(20.0_f64.to_bits().to_string()), stopped_event: None };
        let insert = |second: u32, from: &str, data| {
            db.insert_event_row("message", from, &data, Some(&format!("1970-01-01T00:00:{second:02}.000000+00:00"))).unwrap()
        };
        let old = insert(11, name, json!({"from":name,"text":"old"}));
        let stop = db.insert_event_row("life", name, &json!({"action":"stopped","snapshot":{"created_at":10.0,"tool":"codex"}}), Some("1970-01-01T00:00:12.000000+00:00")).unwrap();
        let historical = AgentRef { name: name.into(), incarnation: None, stopped_event: Some(stop.to_string()) };
        let first = insert(20, name, json!({"from":"display alias","text":"outgoing","delivered_to":[name,name,"other"]}));
        db.conn().execute_batch(r#"WITH RECURSIVE seed(n) AS (VALUES (1) UNION ALL SELECT n+1 FROM seed WHERE n<3000)
            INSERT INTO events(timestamp,type,instance,data) SELECT '1970-01-01T00:00:21.000000+00:00','message','unrelated','{"delivered_to":["other"]}' FROM seed;"#).unwrap();
        let second = insert(22, "origin", json!({"from":"origin","text":"incoming","delivered_to":[name],"thread":"thread-a","reply_to":first}));
        insert(23, "origin", json!({"from":name,"text":"not actually delivered","scope":"mentions","mentions":[name]}));

        assert!(db.start_conversation_routes().unwrap());
        assert!(!db.conversation_routes_cover(0, i64::MAX));
        // New writes made during migration are indexed immediately, but the
        // first page still consults the journal until its window is covered.
        let third = insert(24, "origin", json!({"from":"origin","text":"new while backfilling","delivered_to":[name]}));
        assert!(db.conversation_routes_cover(second.max(third - 1), i64::MAX));
        let encode = |page| serde_json::to_value(page).unwrap();
        let legacy = encode(messages(db, &context, agent.clone(), None, 20).unwrap());
        assert_eq!(legacy["messages"].as_array().unwrap().len(), 3);
        assert!(db.with_read_budget(|| true, || messages(db, &context, agent.clone(), None, 20)).is_err());
        let row = db.get_instance_full(name).unwrap().unwrap();
        let count = |instance| crate::service::queries_store::listing::try_get_unread_counts_batch(db, &[instance]).unwrap()[name];
        let legacy_count = count(row.clone());
        assert_eq!(legacy_count, 3, "only actual delivery counts, even with duplicates and aliases");
        assert!(db.backfill_conversation_routes().unwrap());
        assert!(db.conversation_routes_cover(0, stop));
        assert!(!db.conversation_routes_cover(0, i64::MAX));
        assert_eq!(messages(db, &context, historical.clone(), None, 20).unwrap().messages[0].id, old.to_string());
        while db.backfill_conversation_routes().unwrap() {}
        assert!(db.conversation_routes_cover(0, i64::MAX));

        // The indexed read fits before the first 1,000-instruction budget check.
        let fast = db.with_read_budget(|| true, || messages(db, &context, agent.clone(), None, 20)).unwrap().unwrap();
        assert_eq!(encode(fast), legacy);
        assert_eq!(db.with_read_budget(|| true, || count(row.clone())).unwrap(), legacy_count);
        let after_first = InstanceRow { last_event_id: first, ..row };
        assert_eq!(count(after_first), 2);
        assert_eq!(db.with_read_budget(|| true, || projected_agent(db, &context, &agent)).unwrap().unwrap().pending_messages, Some(3));
        let page = messages(db, &context, agent.clone(), None, 1).unwrap();
        assert_eq!(page.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), [first.to_string()]);
        assert!(page.has_more);
        let continuation = messages(db, &context, agent.clone(), Some(page.next_after), 2).unwrap();
        assert_eq!(continuation.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), [second.to_string(), third.to_string()]);
        assert!(!continuation.has_more);
        assert_eq!(continuation.messages[0].reply_to, Some(first.to_string()));
        assert_eq!(continuation.messages[0].thread.as_deref(), Some("thread-a"));

        let fourth = insert(25, name, json!({"from":name,"text":"new arrival"}));
        assert_eq!(messages(db, &context, agent.clone(), Some(continuation.next_after), 20).unwrap().messages[0].id, fourth.to_string());
        db.conn().execute("UPDATE events SET instance='other',data='{}' WHERE id=?1", [fourth]).unwrap();
        assert!(messages(db, &context, agent.clone(), Some(third.to_string()), 20).unwrap().messages.is_empty());
        db.conn().execute("DELETE FROM events WHERE id=?1", [third]).unwrap();
        assert!(messages(db, &context, agent, Some(second.to_string()), 20).unwrap().messages.is_empty());
        db.with_write_scope(|| db.clear_managed_state_for_reset()).unwrap();
        let rows: i64 = db.conn().query_row("SELECT COUNT(*) FROM conversation_routes", [], |row| row.get(0)).unwrap();
        assert_eq!(rows, 0);
    });
}

#[test]
fn durable_attachment_only_send_preserves_projection_replay_and_identity() {
    with_db(|db, context| {
        db.save_instance_named("toku", json!({"created_at":10.0,"tool":"codex","status":"active","pid":context.process.pid}).as_object().unwrap()).unwrap();
        let agent = AgentRef { name: "toku".into(), incarnation: Some(10.0_f64.to_bits().to_string()), stopped_event: None };
        let stored = super::super::uploads::persist(db.path().parent().unwrap(), &context.database_generation, "../picture.png", "image/png", b"image fixture").unwrap();
        handle(db, &context, Request::RegisterAttachment(stored.clone())).unwrap();
        let message: MessageInput = serde_json::from_value(json!({"text":"", "attachments":[stored.attachment.id], "intent":"inform"})).unwrap();
        let send = || handle(db, &context, Request::Send { agent: agent.clone(), message: message.clone() }).unwrap();
        let Response::Sent(first) = send() else { panic!("expected send") };
        let Response::Sent(replay) = send() else { panic!("expected replay") };
        assert_eq!(first.message_id, replay.message_id);
        let page = messages(db, &context, agent.clone(), None, 50).unwrap();
        assert_eq!(page.messages.len(), 1);
        assert_eq!(page.messages[0].attachments, vec![stored.attachment.clone()]);
        assert!(page.messages[0].text.contains(&stored.path));
        assert_eq!(std::fs::read(&stored.path).unwrap(), b"image fixture");
        let reopened = AgentxDb::open_at(db.path()).unwrap();
        assert_eq!(messages(&reopened, &context, agent.clone(), None, 50).unwrap().messages[0].attachments[0], stored.attachment);
        let missing = MessageInput { attachments: vec![uuid::Uuid::new_v4().to_string()], ..message.clone() };
        assert!(validate_command(db, &context.database_generation, &Command::Send { agent: agent.clone(), message: missing }).is_err());
        assert!(attachment(db, &uuid::Uuid::new_v4().to_string(), &stored.attachment.id).is_err());
        let stale = AgentRef { incarnation: Some(11.0_f64.to_bits().to_string()), ..agent };
        assert!(handle(db, &context, Request::Send { agent: stale, message }).is_err());
    });
}

#[test]
fn attachment_bounds_and_legacy_text_are_explicit() {
    with_db(|db, context| {
        let legacy: MessageInput = serde_json::from_value(json!({"text":"legacy text"})).unwrap();
        assert!(legacy.attachments.is_empty());
        validate_message(&legacy).unwrap();
        let empty = MessageInput { text: "".into(), ..legacy.clone() };
        assert!(validate_message(&empty).is_err());
        let too_many = MessageInput { attachments: (0..9).map(|_| uuid::Uuid::new_v4().to_string()).collect(), ..legacy };
        assert!(resolve_attachments(db, &context.database_generation, &too_many).is_err());
    });
}

#[test]
fn interaction_commands_use_native_capabilities_and_exact_identity() {
    with_db(|db, context| {
        db.save_instance_named("menu", json!({"created_at":10.0,"tool":"codex","status":"active","session_id":"fixture-session"}).as_object().unwrap()).unwrap();
        let reference = AgentRef { name: "menu".into(), incarnation: Some(10.0_f64.to_bits().to_string()), stopped_event: None };
        let view = agent(db, &reference).unwrap();
        assert!(view.can_fork && view.can_stop_participation);
        validate_command(db, &context.database_generation, &Command::Fork(reference.clone())).unwrap();
        validate_command(db, &context.database_generation, &Command::StopParticipation(reference.clone())).unwrap();
        assert!(result_matches(&Command::Fork(reference.clone()), &launched("child")));
        assert!(result_matches(&Command::StopParticipation(reference.clone()), &CommandResult::Stopped(StopOutcome { name: "menu".into(), outcome: "participation_stopped".into() })));
        let stale = AgentRef { incarnation: Some(11.0_f64.to_bits().to_string()), ..reference.clone() };
        assert!(validate_command(db, &context.database_generation, &Command::Fork(stale)).is_err());
        db.conn().execute("UPDATE instances SET tool='gemini' WHERE name='menu'", []).unwrap();
        assert!(!agent(db, &reference).unwrap().can_fork);
        assert!(validate_command(db, &context.database_generation, &Command::Fork(reference)).is_err());
        assert!(!can_fork("codex", None));
        assert!(!can_fork("unknown-provider", Some("session")));
    });
}

#[test]
fn labels_inline_journal_replay_and_conflict() {
    with_db(|db, context| {
        db.conn().execute("INSERT INTO instances(name,tool,created_at) VALUES ('luna','codex',1.0)", []).unwrap();
        let agent_ref = AgentRef { name: "luna".into(), incarnation: Some(1.0f64.to_bits().to_string()), stopped_event: None };
        let command = Command::PatchLabels(LabelsPatch { agent: agent_ref.clone(), set: [("project".into(), "atlas".into())].into(), remove: vec![], if_revision: Some("0".into()) });
        let id = uuid::Uuid::new_v4().to_string();
        let first = submit(db, &context, &id, command.clone(), false).unwrap();
        assert!(first.ticket.is_none());
        assert!(matches!(first.operation.status, OperationStatus::Succeeded));
        let Some(CommandResult::LabelsPatched(first)) = first.operation.result else { panic!("expected labels result") };
        assert!(first.changed);
        let replay = submit(db, &context, &id, command.clone(), false).unwrap();
        let Some(CommandResult::LabelsPatched(replay)) = replay.operation.result else { panic!("expected replay") };
        assert_eq!(first.snapshot.revision, replay.snapshot.revision);
        let conflicting = Command::PatchLabels(LabelsPatch { agent: agent_ref.clone(), set: [("review/state".into(), "approved".into())].into(), remove: vec![], if_revision: Some("0".into()) });
        assert!(matches!(submit(db, &context, &uuid::Uuid::new_v4().to_string(), conflicting.clone(), true), Err(ServiceError::Conflict(_))));
        assert!(matches!(submit(db, &context, &id, conflicting, true), Err(ServiceError::Conflict(_))));
        let Response::Labels(snapshot) = handle(db, &context, Request::Labels(agent_ref)).unwrap() else { panic!("expected snapshot") };
        assert_eq!(snapshot.labels.len(), 1);
        assert_eq!(snapshot.labels["project"], "atlas");
    });
}

#[test]
fn labels_wire_rejects_duplicate_keys_before_map_deserialization() {
    let bytes = br#"{"request_id":"test","generation":"test","agent":{"name":"luna","incarnation":"1","stopped_event":null},"set":{"project":"a","project":"b"},"remove":[],"if_revision":null}"#;
    assert!(crate::browser::wire::decode_json::<SubmitLabels>(bytes).is_err());
}

#[test]
fn runtime_read_batch_preserves_recipient_cursors_zero_and_exact_identity() {
    with_db(|db, context| {
        for (name, origin) in [("early", None), ("later", None), ("empty", None), ("remote", Some("other-device"))] {
            db.conn().execute("INSERT INTO instances(name,tool,created_at,origin_device_id) VALUES (?1,'codex',10.0,?2)", rusqlite::params![name, origin]).unwrap();
        }
        let first = db.log_event("message", "sender", &json!({"delivered_to":["early","later","remote"]})).unwrap();
        db.log_event("message", "sender", &json!({"delivered_to":["early","early","later","remote","absent"]})).unwrap();
        db.log_event("status", "sender", &json!({"delivered_to":["early","empty"]})).unwrap();
        db.log_event("message", "sender", &json!({"delivered_to":[]})).unwrap();
        db.conn().execute("UPDATE instances SET last_event_id=?1 WHERE name='later'", [first]).unwrap();
        let rows = db.iter_instances_full().unwrap();
        let counts = crate::service::queries_store::listing::try_get_unread_counts_batch(db, &rows).unwrap();
        assert_eq!(counts.get("early").copied().unwrap_or(0), 2);
        assert_eq!(counts.get("later").copied().unwrap_or(0), 1);
        assert_eq!(counts.get("empty").copied().unwrap_or(0), 0);
        assert!(!counts.contains_key("remote"));
        assert!(crate::service::queries_store::listing::try_get_unread_counts_batch(db, &[]).unwrap().is_empty());
        for request in [Request::List { after: None, limit: 100 }, Request::SelectLabels { after: None, limit: 100, selectors: vec![] }] {
            let Response::Agents(page) = handle(db, &context, request).unwrap() else { panic!("expected Agent page") };
            assert_eq!(page.agents.len(), 3);
            for (name, expected) in [("early", 2), ("later", 1), ("empty", 0)] {
                let view = page.agents.iter().find(|view| view.agent.name == name).unwrap();
                assert_eq!(view.pending_messages, Some(expected));
                assert_eq!(view.agent.incarnation, Some(10.0_f64.to_bits().to_string()));
            }
        }
        let stale = AgentRef { name: "early".into(), incarnation: Some(11.0_f64.to_bits().to_string()), stopped_event: None };
        assert!(matches!(agent(db, &stale), Err(ServiceError::StaleFence)));
    });
}

#[test]
fn runtime_read_batch_database_failure_is_not_reported_as_zero() {
    with_db(|db, context| {
        db.conn().execute("INSERT INTO instances(name,tool,created_at) VALUES ('early','codex',10.0)", []).unwrap();
        let rows = db.iter_instances_full().unwrap();
        // Corrupt only the disposable schema: distinguish SQL failure from an empty unread tail.
        db.conn().execute("ALTER TABLE events RENAME TO unavailable_events", []).unwrap();
        assert!(matches!(crate::service::queries_store::listing::try_get_unread_counts_batch(db, &rows), Err(ServiceError::Internal(_))));
        for request in [Request::List { after: None, limit: 100 }, Request::SelectLabels { after: None, limit: 100, selectors: vec![] }] {
            assert!(handle(db, &context, request).is_err());
        }
    });
}

#[test]
fn runtime_read_eligibility_skips_unread_but_detail_remains_fallible() {
    with_db(|db, context| {
        db.save_instance_named("ready", json!({"tool":"codex","status":"active","created_at":10.0}).as_object().unwrap()).unwrap();
        let reference = AgentRef { name: "ready".into(), incarnation: Some(10.0_f64.to_bits().to_string()), stopped_event: None };
        db.log_event("message", "other", &json!({"delivered_to":"not-json-array"})).unwrap();
        let raw = agent(db, &reference).unwrap();
        assert!(raw.can_send);
        assert_eq!(raw.pending_messages, None);
        let message = MessageInput { text: "validate only, never submit".into(), attachments: vec![], intent: None, reply_to: None, thread: None };
        validate_command(db, &context.database_generation, &Command::Send { agent: reference.clone(), message }).unwrap();
        assert!(matches!(projected_agent(db, &context, &reference), Err(ServiceError::Internal(_))));
        db.conn().execute("DELETE FROM events WHERE type='message'", []).unwrap();
        assert_eq!(projected_agent(db, &context, &reference).unwrap().pending_messages, Some(0));
    });
}

fn adopted_projection(db: &AgentxDb, context: &RequestContext) -> AdoptedWorkspaces {
    let Response::AdoptedWorkspaces(projection) = handle(db, context, Request::AdoptedWorkspaces).unwrap() else { panic!("expected compact adopted workspaces") };
    projection
}

#[test]
fn runtime_read_compact_workspaces_retain_history_without_agent_hydration() {
    with_db(|db, context| {
        let collaboration = |path: &str| json!({"collaboration":{"workspace":path,"role":null,"endpoint":false}});
        db.save_instance_named("same", json!({"tool":"codex","created_at":2.0,"launch_context":collaboration("/current").to_string()}).as_object().unwrap()).unwrap();
        db.save_instance_named("remote", json!({"tool":"codex","created_at":2.0,"origin_device_id":"elsewhere","launch_context":collaboration("/remote").to_string()}).as_object().unwrap()).unwrap();
        for launch_context in [collaboration("/historical-object"), json!(collaboration("/historical-string").to_string())] {
            // Invalid labels would reject a full historical AgentView. Membership needs only collaboration.
            db.log_event("life", "same", &json!({"action":"stopped","snapshot":{"created_at":1.0,"launch_context":launch_context,"labels":["invalid-for-AgentView"]}})).unwrap();
        }
        db.log_event("life", "same", &json!({"action":"stopped","snapshot":{"launch_context":collaboration("/current")}})).unwrap();
        db.log_event("life", "remote", &json!({"action":"stopped","snapshot":{"origin_device_id":"elsewhere","launch_context":collaboration("/remote-history")}})).unwrap();
        db.log_event("life", "legacy", &json!({"action":"stopped","snapshot":{"launch_context":{"collaboration":{"workspace":"relative","role":null,"endpoint":false}}}})).unwrap();
        // Valid JSON satisfies event indexes; its malformed recipient scalar must not be read by compact membership projection.
        db.log_event("message", "sender", &json!({"delivered_to":"not-json-array"})).unwrap();
        let projection = adopted_projection(db, &context);
        let paths: std::collections::BTreeSet<_> = projection.paths.iter().map(String::as_str).collect();
        assert_eq!(paths, ["/current", "/historical-object", "/historical-string"].into());
        assert_eq!(projection.paths, ["/current", "/historical-string", "/historical-object"], "dedup preserves current-first and stopped-desc admission order");
        assert_eq!(projection.agent_pages, 1);
        assert!(!projection.agent_limit_reached);
        assert!(list(db, &context, None, 100).is_err());
    });
}

#[test]
fn runtime_read_compact_workspaces_keep_first_thousand_coverage() {
    with_db(|db, context| {
        let empty = adopted_projection(db, &context);
        assert!(empty.paths.is_empty());
        assert_eq!(empty.agent_pages, 1);
        assert!(!empty.agent_limit_reached);
        // Two current rows take precedence over descending stopped executions.
        for name in ["aaaa", "zzzz"] {
            db.save_instance_named(name, json!({"tool":"codex","created_at":2.0,"launch_context":json!({"collaboration":{"workspace":format!("/{name}"),"role":null,"endpoint":false}}).to_string()}).as_object().unwrap()).unwrap();
        }
        for index in 0..998 {
            db.log_event("life", "history", &json!({"action":"stopped","snapshot":{"launch_context":{"collaboration":{"workspace":format!("/history-{index}"),"role":null,"endpoint":false}}}})).unwrap();
        }
        let full = adopted_projection(db, &context);
        assert_eq!(full.agent_pages, 10);
        assert!(!full.agent_limit_reached);
        assert_eq!(full.paths.len(), 1000);
        assert_eq!(&full.paths[..3], ["/aaaa", "/zzzz", "/history-997"]);
        db.log_event("life", "history", &json!({"action":"stopped","snapshot":{"launch_context":{"collaboration":{"workspace":"/newest","role":null,"endpoint":false}}}})).unwrap();
        let truncated = adopted_projection(db, &context);
        assert_eq!(truncated.agent_pages, 10);
        assert!(truncated.agent_limit_reached);
        assert_eq!(truncated.paths.len(), 1000);
        for included in ["/aaaa", "/zzzz", "/newest", "/history-997"] {
            assert!(truncated.paths.iter().any(|path| path == included), "missing {included}");
        }
        assert!(!truncated.paths.iter().any(|path| path == "/history-0"));
    });
}

#[test]
fn message_history_wal_snapshot_pins_coverage_and_watermark_during_append_and_backfill() {
    with_db(|db, context| {
        let name = "target";
        db.save_instance_named(name, json!({"created_at":20.0,"tool":"codex"}).as_object().unwrap()).unwrap();
        let current = AgentRef { name: name.into(), incarnation: Some(20.0_f64.to_bits().to_string()), stopped_event: None };
        let insert = |second: u32, routing, data| {
            db.insert_event_row("message", routing, &data, Some(&format!("1970-01-01T00:00:{second:02}.000000+00:00"))).unwrap()
        };
        let historical_id = insert(10, name, json!({"text":"before resume"}));
        let stop = db.insert_event_row("life", name, &json!({"action":"stopped","snapshot":{"created_at":10.0,"tool":"codex"}}), Some("1970-01-01T00:00:12.000000+00:00")).unwrap();
        let stopped = AgentRef { name: name.into(), incarnation: None, stopped_event: Some(stop.to_string()) };
        let first = insert(20, name, json!({"text":"outgoing"}));
        let second = insert(21, "sender", json!({"text":"actual delivery","delivered_to":[name,name]}));
        let watermark = insert(22, "sender", json!({"text":"mention only","mentions":[name],"from":name}));
        assert!(db.start_conversation_routes().unwrap());

        let reader = AgentxDb::open_reader(db.path()).unwrap();
        let snapshot = reader.conn().unchecked_transaction().unwrap();
        let pinned = reader.read_database_generation().unwrap();
        assert_eq!(pinned, context.database_generation);
        assert!(!reader.conversation_routes_cover(0, watermark));
        while db.backfill_conversation_routes().unwrap() {}
        let new = insert(23, name, json!({"text":"after the snapshot"}));
        assert!(!reader.conversation_routes_cover(0, watermark));

        let latest = message_history(&reader, &context, current.clone(), None, 1).unwrap();
        assert_eq!(latest.messages[0].id, second.to_string());
        assert_eq!(latest.next_before, Some(second.to_string()));
        assert_eq!(latest.live_after, Some(watermark.to_string()));
        let older = message_history(&reader, &context, current.clone(), latest.next_before, 1).unwrap();
        assert_eq!(older.messages[0].id, first.to_string());
        assert!(older.next_before.is_none());
        assert!(older.live_after.is_none());
        let stopped = message_history(&reader, &context, stopped, None, 20).unwrap();
        assert_eq!(stopped.messages.into_iter().map(|m| m.id).collect::<Vec<_>>(), [historical_id.to_string()]);
        drop(snapshot);

        // Once the snapshot ends, the indexed route sees the append. The same
        // append is consumed exactly once from the original forward watermark.
        let page = reader.with_read_budget(|| true, || message_history(&reader, &context, current.clone(), None, 20)).unwrap().unwrap();
        assert_eq!(page.messages.into_iter().map(|m| m.id).collect::<Vec<_>>(), [first, second, new].map(|id| id.to_string()));
        assert_eq!(messages(&reader, &context, current, latest.live_after, 20).unwrap().messages.into_iter().map(|m| m.id).collect::<Vec<_>>(), [new.to_string()]);
    });
}

#[test]
fn message_history_raw_cap_cannot_claim_false_exhaustion() {
    with_db(|db, context| {
        let name = "target";
        db.save_instance_named(name, json!({"created_at":20.0,"tool":"codex"}).as_object().unwrap()).unwrap();
        let current = AgentRef { name: name.into(), incarnation: Some(20.0_f64.to_bits().to_string()), stopped_event: None };
        let time = Some("1970-01-01T00:00:20.000000+00:00");
        let oldest = db.insert_event_row("message", name, &json!({"text":"before the raw scan window"}), time).unwrap();
        db.conn().execute_batch(r#"WITH RECURSIVE seed(n) AS (VALUES (1) UNION ALL SELECT n+1 FROM seed WHERE n<4096)
            INSERT INTO events(timestamp,type,instance,data) SELECT '1970-01-01T00:00:20.000000+00:00','message','unrelated','{}' FROM seed;"#).unwrap();
        assert!(db.start_conversation_routes().unwrap());
        assert!(matches!(message_history(db, &context, current.clone(), None, 1), Err(ServiceError::Unavailable(_))));
        let first = db.insert_event_row("message", name, &json!({"text":"in budget"}), time).unwrap();
        let newest = db.insert_event_row("message", name, &json!({"text":"latest"}), time).unwrap();
        let page = message_history(db, &context, current.clone(), None, 1).unwrap();
        assert_eq!(page.messages[0].id, newest.to_string());
        assert_eq!(page.next_before, Some(newest.to_string()));
        assert!(matches!(message_history(db, &context, current.clone(), page.next_before.clone(), 1), Err(ServiceError::Unavailable(_))));

        while db.backfill_conversation_routes().unwrap() {}
        let page = db.with_read_budget(|| true, || message_history(db, &context, current, page.next_before, 2)).unwrap().unwrap();
        assert_eq!(page.messages.into_iter().map(|m| m.id).collect::<Vec<_>>(), [oldest, first].map(|id| id.to_string()));
        assert!(page.next_before.is_none());
        assert!(page.live_after.is_none());
    });
}

#[test]
fn message_history_byte_limit_resumes_at_the_unread_message() {
    with_db(|db, context| {
        let name = "target";
        db.save_instance_named(name, json!({"created_at":20.0,"tool":"codex"}).as_object().unwrap()).unwrap();
        let current = AgentRef { name: name.into(), incarnation: Some(20.0_f64.to_bits().to_string()), stopped_event: None };
        db.start_conversation_routes().unwrap();
        let data = json!({"text":"x".repeat(crate::events::service::MAX_PAGE_BYTES / 2)});
        let time = Some("1970-01-01T00:00:20.000000+00:00");
        let older = db.insert_event_row("message", name, &data, time).unwrap();
        let newer = db.insert_event_row("message", name, &data, time).unwrap();

        let page = message_history(db, &context, current.clone(), None, 20).unwrap();
        assert_eq!(page.messages.into_iter().map(|m| m.id).collect::<Vec<_>>(), [newer.to_string()]);
        let page = message_history(db, &context, current, page.next_before, 20).unwrap();
        assert_eq!(page.messages.into_iter().map(|m| m.id).collect::<Vec<_>>(), [older.to_string()]);
        assert!(page.next_before.is_none());
        assert!(page.live_after.is_none());
    });
}
