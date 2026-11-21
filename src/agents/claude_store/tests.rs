//! Native Claude scenarios. Integration executes these; no host effect is
//! applied, and all files/rows belong to a disposable fixture scope.
use super::*;
use crate::messaging::inbox::{DeliveryEvidence, DeliveryReceipt};
use crate::messaging::service::{OutputConfirmation, PreparedOutput};
use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};

fn fixture(body: impl FnOnce(&AgentxDb, &RequestContext)) {
    let directory = tempfile::Builder::new().prefix("agentx-claude-policy-").tempdir().unwrap();
    let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
    let context = RequestContext {
        caller: CallerContext {
            request_id: RequestId("fixture".into()),
            invocation: InvocationContext { cwd: directory.path().join("origin"), process_binding_id: Some("worker".into()), ..Default::default() },
            deadline_unix_ms: None,
            expected_database_generation: None,
        },
        process: CallerProcess { pid: std::process::id(), incarnation: crate::sys::process::identity(std::process::id()) },
        daemon_instance: "claude-test".into(),
        database_generation: db.database_generation().unwrap(),
    };
    assert!(context.process.incarnation.is_some());
    body(&db, &context);
}
fn request(context: &RequestContext, id: &str) -> RequestContext {
    let mut next = context.clone();
    next.caller.request_id = RequestId(id.to_owned());
    next
}
fn seed(db: &AgentxDb, name: &str, session: &str, created: f64) -> InstanceRow {
    db.save_instance_named(name, json!({"tool":"claude","session_id":session,"created_at":created,"status":"listening","status_context":"start","directory":"/old","last_event_id":0}).as_object().unwrap()).unwrap();
    db.set_session_binding(session, name).unwrap();
    db.get_instance_full(name).unwrap().unwrap()
}
fn root_fixture(db: &AgentxDb) -> Actor {
    let root = seed(db, "root", "session", 1.0);
    db.set_process_binding("worker", "session", "root").unwrap();
    db.mark_claude_session_validated("session", "root").unwrap();
    Actor { root: participant(&root), participant: participant(&root), session_id: "session".into(), agent_id: None }
}
fn child_fixture(db: &AgentxDb, root: &Actor, name: &str, id: &str, created: f64) -> Actor {
    db.save_instance_named(
        name,
        json!({"tool":"claude","created_at":created,"parent_name":root.root.canonical_name,"parent_session_id":root.session_id,"agent_id":id,"status":"inactive","status_context":"subagent:dormant","directory":"/child","last_event_id":0})
            .as_object()
            .unwrap(),
    )
    .unwrap();
    Actor { root: root.root.clone(), participant: participant(&db.get_instance_full(name).unwrap().unwrap()), session_id: root.session_id.clone(), agent_id: Some(id.into()) }
}
fn observation(kind: ObservationKind, session: &str) -> Observation {
    Observation { kind, session_id: session.into(), env_session_candidate: None, agent_id: None, transcript_path: None, directory: None, evidence: TranscriptEvidence::NotRead, terminal_shell_tool_use: None }
}
fn observed(db: &AgentxDb, context: &RequestContext, input: Observation) -> Resolution {
    match handle(db, context, Request::Observe(input)).unwrap() {
        Response::Observed(result) => result,
        _ => panic!("wrong observation response"),
    }
}
fn output(db: &AgentxDb, context: &RequestContext, actor: &Actor, bootstrap: bool) -> Output {
    match handle(db, context, Request::PrepareOutput { actor: actor.clone(), mailbox: true, bootstrap, stop_claim: None }).unwrap() {
        Response::Output(output) => output,
        _ => panic!("wrong output response"),
    }
}
fn receipt(output: &PreparedOutput) -> Option<OutputConfirmation> {
    match output {
        PreparedOutput::Unmanaged { receipt, messages, .. } if !messages.is_empty() => Some(OutputConfirmation::Unmanaged(receipt.clone())),
        PreparedOutput::Claimed { target, batch, messages, .. } if !messages.is_empty() => {
            Some(OutputConfirmation::Claimed(DeliveryReceipt { target: target.clone(), batch: batch.id.clone(), through: batch.window.through, evidence: DeliveryEvidence::HookOutputWritten }))
        }
        _ => None,
    }
}
fn message(db: &AgentxDb, from: &str, recipients: &[&str], text: &str) -> i64 {
    db.log_event("message", from, &json!({"from":from,"text":text,"scope":"mentions","mentions":recipients})).unwrap()
}
fn begin_child(db: &AgentxDb, context: &RequestContext, root: &Actor, id: &str) -> ChildStopResult {
    match handle(db, context, Request::BeginChildStop { root: root.clone(), agent_id: id.into(), prompt_id: "prompt".into(), fingerprint: "0123456789abcdef".into(), transcript_path: Some("/origin/child.jsonl".into()) }).unwrap() {
        Response::ChildStop(result) => result,
        _ => panic!("wrong child response"),
    }
}

#[test]
fn fresh_fork_keeps_reserved_identity_and_persistent_flag_cannot_override_later_start() {
    fixture(|db, context| {
        seed(db, "ancestor", "ancestor-session", 1.0);
        db.save_instance_named("reserved", json!({"tool":"claude","created_at":2.0,"status":"pending","status_context":"new","last_event_id":0}).as_object().unwrap()).unwrap();
        db.conn().execute("INSERT INTO process_bindings(process_id,instance_name,session_id,updated_at) VALUES('worker','reserved',NULL,0)", []).unwrap();
        let mut context = context.clone();
        context.caller.invocation.environment.insert("AGENTX_IS_FORK".into(), "1".into());
        context.caller.invocation.environment.insert("AGENTX_LAUNCHED".into(), "1".into());
        let session = "12345678-1234-1234-1234-123456789abc";
        let mut input = observation(ObservationKind::SessionStart { source: StartSource::Fork }, "ancestor-session");
        input.env_session_candidate = Some(session.into());
        input.evidence = TranscriptEvidence::Envelopes { top_level_sessions: vec![], message_sessions: vec!["ancestor-session".into()], bytes_read: 80, records_read: 1 };
        let Resolution::Ready(view) = observed(db, &request(&context, "fresh-fork"), input.clone()) else { panic!("fresh fork rejected") };
        assert_eq!(view.actor.participant.canonical_name, "reserved");
        assert_eq!(view.actor.session_id, session);
        assert_eq!(db.get_instance_full("ancestor").unwrap().unwrap().session_id.as_deref(), Some("ancestor-session"));
        assert_eq!(db.get_instance_full("reserved").unwrap().unwrap().name_announced, 0);
        input.session_id = "different-native-session".into();
        input.evidence = TranscriptEvidence::Missing;
        assert!(matches!(observed(db,&request(&context,"later-start"),input),Resolution::Unbound {session_id} if session_id=="different-native-session"));
        assert_eq!(db.get_process_binding_full("worker").unwrap().unwrap().0.as_deref(), Some(session));
    });
}

#[test]
fn lineage_envelopes_exclude_unvalidated_top_level_and_distinguish_ambiguity_and_read_failure() {
    fixture(|db, context| {
        root_fixture(db);
        seed(db, "foreign", "claimed", 2.0);
        db.clear_claude_session_validation("session").unwrap();
        let input = observation(ObservationKind::SessionStart { source: StartSource::Resume }, "claimed");
        let before = db.get_last_event_id();
        assert!(matches!(observed(db, &request(context, "need-lineage"), input.clone()), Resolution::NeedsLineage { .. }));
        let mut claim = input.clone();
        claim.evidence = TranscriptEvidence::Envelopes { top_level_sessions: vec!["claimed".into()], message_sessions: vec![], bytes_read: 50, records_read: 1 };
        assert!(matches!(observed(db, &request(context, "self-top-level"), claim), Resolution::Unbound { .. }));
        let mut ambiguous = input.clone();
        ambiguous.evidence = TranscriptEvidence::Envelopes { top_level_sessions: vec!["session".into()], message_sessions: vec!["claimed".into()], bytes_read: 100, records_read: 2 };
        assert!(matches!(observed(db, &request(context, "ambiguous"), ambiguous), Resolution::Ambiguous { .. }));
        let mut failed = input.clone();
        failed.evidence = TranscriptEvidence::ReadFailed;
        assert!(matches!(handle(db, &request(context, "read-error"), Request::Observe(failed)), Err(ServiceError::Unavailable(_))));
        assert_eq!(db.get_last_event_id(), before);
        assert_eq!(db.get_instance_full("root").unwrap().unwrap().session_id.as_deref(), Some("session"));
        let mut proven = input;
        proven.evidence = TranscriptEvidence::Envelopes { top_level_sessions: vec!["claimed".into()], message_sessions: vec!["session".into()], bytes_read: 100, records_read: 1 };
        let Resolution::Ready(view) = observed(db, &request(context, "proven"), proven) else { panic!("structured lineage rejected") };
        assert_eq!(view.actor.participant.canonical_name, "root");
        assert_eq!(db.get_validated_claude_session_owner("claimed").unwrap().as_deref(), Some("root"));
    });
}

#[test]
fn compact_recovery_ignores_file_read_failure_and_bootstrap_ready_waits_for_flush() {
    fixture(|db, context| {
        let actor = root_fixture(db);
        let mut context = context.clone();
        context.caller.invocation.environment.extend([("AGENTX_LAUNCHED".into(), "1".into()), ("AGENTX_LAUNCH_BATCH_ID".into(), "batch".into())]);
        let mut input = observation(ObservationKind::SessionStart { source: StartSource::Compact }, "session");
        input.evidence = TranscriptEvidence::ReadFailed;
        let Resolution::Ready(view) = observed(db, &request(&context, "compact"), input) else { panic!("compact rejected") };
        let receipt = view.bootstrap.unwrap().receipt;
        let count = || db.conn().query_row("SELECT count(*) FROM events WHERE type='life' AND json_extract(data,'$.action')='ready'", [], |row| row.get::<_, i64>(0)).unwrap();
        assert_eq!(count(), 0);
        assert_eq!(db.get_instance_full("root").unwrap().unwrap().name_announced, 0);
        let confirm = Request::ConfirmFlushed { actor, mailbox: None, bootstrap: Some(receipt), history: None };
        let mut stolen = request(&context, "stolen");
        stolen.process.incarnation = Some("wrong-peer".into());
        assert!(matches!(handle(db, &stolen, confirm.clone()), Err(ServiceError::StaleFence)));
        handle(db, &request(&context, "flush"), confirm.clone()).unwrap();
        assert_eq!(count(), 1);
        handle(db, &request(&context, "flush"), confirm).unwrap();
        assert_eq!(count(), 1);
        assert_eq!(db.get_instance_full("root").unwrap().unwrap().name_announced, 1);
    });
}

#[test]
fn child_routes_to_root_owned_exact_identity_and_never_adopts_foreign_agent() {
    fixture(|db, context| {
        let root = root_fixture(db);
        let mut input = observation(ObservationKind::SubagentStart { agent_type: "reviewer".into() }, "session");
        input.agent_id = Some("agent-one".into());
        input.directory = Some("/originating-workspace".into());
        let Resolution::Ready(view) = observed(db, &request(context, "child-start"), input.clone()) else { panic!("child rejected") };
        assert_eq!(view.actor.root, root.root);
        assert_eq!(view.record.parent_session_id.as_deref(), Some("session"));
        assert_eq!(view.record.directory, "/originating-workspace");
        assert_eq!(view.record.name_announced, 0);
        let before = db.get_last_event_id();
        handle(db, &request(context, "child-tool"), Request::Activity { actor: view.actor.clone(), activity: Activity::PermissionRequested { tool_name: "Bash".into(), detail: "approval detail".into(), tool_use_id: Some("tool".into()) } }).unwrap();
        assert_eq!(db.get_instance_full(&root.root.canonical_name).unwrap().unwrap().status, "listening");
        assert_eq!(db.get_instance_full(&view.record.name).unwrap().unwrap().status, "blocked");
        assert!(db.get_last_event_id() > before);
        let other = seed(db, "other", "other-session", 3.0);
        db.set_process_binding("worker", "other-session", "other").unwrap();
        db.mark_claude_session_validated("other-session", "other").unwrap();
        input.session_id = "other-session".into();
        assert!(matches!(handle(db, &request(context, "cross-root"), Request::Observe(input)), Err(ServiceError::StaleFence)));
        assert_eq!(db.get_instance_full(&view.record.name).unwrap().unwrap().parent_name.as_deref(), Some("root"));
        assert_ne!(participant(&other), root.root);
    });
}

#[test]
fn shell_capability_is_exact_and_terminal_revoke_survives_failed_actor_routing() {
    fixture(|db, context| {
        let root = root_fixture(db);
        let child = child_fixture(db, &root, "child", "agent", 2.0);
        let Response::Activity { actor_capability: Some(token) } =
            handle(db, &request(context, "issue"), Request::Activity { actor: child.clone(), activity: Activity::ToolStarted { tool_name: "Bash".into(), detail: "agentx send".into(), tool_use_id: Some("shell".into()), visible_agentx_shell: true } })
                .unwrap()
        else {
            panic!("no capability")
        };
        assert_eq!(db.resolve_claude_actor_capability(&token, "session").unwrap().as_deref(), Some("child"));
        let mut unrelated = context.clone();
        unrelated.caller.invocation.process_binding_id = Some("missing".into());
        let mut terminal = observation(ObservationKind::Callback, "session");
        terminal.agent_id = Some("agent".into());
        terminal.terminal_shell_tool_use = Some("shell".into());
        assert!(matches!(observed(db, &request(&unrelated, "terminal-unbound"), terminal), Resolution::Unbound { .. }));
        assert_eq!(db.resolve_claude_actor_capability(&token, "session").unwrap(), None);
        let token = db.issue_claude_actor_capability("session", "second", Some("agent"), "child").unwrap();
        db.update_instance_fields("child", json!({"created_at":9.0}).as_object().unwrap()).unwrap();
        assert_eq!(db.resolve_claude_actor_capability(&token, "session").unwrap(), None, "same-name child incarnation must not inherit a shell grant");
        assert!(matches!(handle(db, &request(context, "stale-child"), Request::Activity { actor: child, activity: Activity::Idle }), Err(ServiceError::StaleFence)));
    });
}

#[test]
fn dormant_child_broadcast_does_not_keep_alive_and_lost_stop_reply_keeps_frozen_plan() {
    fixture(|db, context| {
        let root = root_fixture(db);
        let child = child_fixture(db, &root, "child", "agent", 2.0);
        db.log_event("message", "sender", &json!({"from":"sender","text":"broadcast","scope":"broadcast"})).unwrap();
        let stop_context = request(context, "dormant-stop");
        let ChildStopResult::Retired(first) = begin_child(db, &stop_context, &root, "agent") else { panic!("dormant not retired") };
        assert!(first.stopped);
        assert_eq!(first.effects[0].participant, child.participant);
        assert_eq!(first.effects[0].record.transcript_path, "/origin/child.jsonl");
        let last = db.get_last_event_id();
        let ChildStopResult::Retired(replay) = begin_child(db, &stop_context, &root, "agent") else { panic!("lost stop reply unavailable") };
        assert_eq!(replay.effects[0].participant, first.effects[0].participant);
        assert_eq!(db.get_last_event_id(), last);
        child_fixture(db, &root, "child", "agent", 9.0);
        let replay = Request::BeginChildStop { root, agent_id: "agent".into(), prompt_id: "prompt".into(), fingerprint: "0123456789abcdef".into(), transcript_path: Some("/origin/child.jsonl".into()) };
        assert!(matches!(handle(db, &stop_context, replay), Err(ServiceError::StaleFence)), "successor must reject retained cleanup");
    });
}

#[test]
fn direct_activation_claim_rejects_duplicate_unknown_and_stolen_owner_then_flushes_exact_child() {
    fixture(|db, context| {
        let root = root_fixture(db);
        let child = child_fixture(db, &root, "child", "agent", 2.0);
        let event = message(db, "sender", &["child"], "direct activation");
        let begin = request(context, "begin");
        let ChildStopResult::Acquired { claim, actor, timeout_secs, dormant, .. } = begin_child(db, &begin, &root, "agent") else { panic!("no claim") };
        assert!(dormant);
        assert_eq!(timeout_secs, 30);
        assert_eq!(actor, child);
        assert!(matches!(begin_child(db, &request(context, "duplicate"), &root, "agent"), ChildStopResult::Duplicate));
        let key: String = db.conn().query_row("SELECT key FROM kv WHERE key LIKE 'subagent_stop_inflight:%'", [], |row| row.get(0)).unwrap();
        let raw = db.kv_get(&key).unwrap().unwrap();
        let mut unknown: serde_json::Value = serde_json::from_str(&raw).unwrap();
        unknown["pid"] = json!(0);
        db.kv_set(&key, Some(&unknown.to_string())).unwrap();
        assert!(matches!(begin_child(db, &request(context, "unknown-owner"), &root, "agent"), ChildStopResult::Duplicate));
        db.kv_set(&key, Some(&raw)).unwrap();
        let mut thief = request(context, "thief");
        thief.process.incarnation = Some("stolen".into());
        assert!(matches!(handle(db, &thief, Request::FinishChildStop { claim: claim.clone(), decision: FinishChildStop::TimedOut }), Err(ServiceError::StaleFence)));
        let prepared = match handle(db, &request(context, "prepare"), Request::PrepareOutput { actor: actor.clone(), mailbox: true, bootstrap: true, stop_claim: Some(claim.clone()) }).unwrap() {
            Response::Output(output) => output,
            _ => panic!("output"),
        };
        assert!(prepared.bootstrap.is_some());
        assert_eq!(db.get_instance_full("child").unwrap().unwrap().last_event_id, 0);
        handle(db, &request(context, "flushed"), Request::ConfirmFlushed { actor: actor.clone(), mailbox: receipt(&prepared.mailbox), bootstrap: prepared.bootstrap.map(|offer| offer.receipt), history: None }).unwrap();
        let row = db.get_instance_full("child").unwrap().unwrap();
        assert_eq!(row.last_event_id, event);
        assert_eq!(row.name_announced, 1);
        assert_eq!(row.status, "active");
        handle(db, &request(context, "release"), Request::FinishChildStop { claim, decision: FinishChildStop::Released }).unwrap();
        assert!(db.kv_get(&key).unwrap().is_none());
        assert_eq!(db.get_instance_full("root").unwrap().unwrap().last_event_id, 0);
    });
}

#[test]
fn child_stop_required_delete_failure_rolls_back_claim_events_and_all_children() {
    fixture(|db, context| {
        let root = root_fixture(db);
        let child = child_fixture(db, &root, "child", "agent", 2.0);
        child_fixture(db, &child, "nested", "nested-agent", 3.0);
        db.update_instance_fields("nested", json!({"parent_name":"child"}).as_object().unwrap()).unwrap();
        let before = db.get_last_event_id();
        db.conn().execute_batch("CREATE TRIGGER fail_child_delete BEFORE DELETE ON instances WHEN OLD.name='child' BEGIN SELECT RAISE(ABORT,'child delete failed'); END;").unwrap();
        let input = Request::BeginChildStop { root: root.clone(), agent_id: "agent".into(), prompt_id: "p".into(), fingerprint: "0123456789abcdef".into(), transcript_path: None };
        assert!(handle(db, &request(context, "stop-fault"), input.clone()).is_err());
        assert!(db.get_instance_full("child").unwrap().is_some());
        assert!(db.get_instance_full("nested").unwrap().is_some());
        assert_eq!(db.get_last_event_id(), before);
        let count: i64 = db.conn().query_row("SELECT count(*) FROM kv WHERE key LIKE 'subagent_stop_inflight:%'", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 0);
        db.conn().execute_batch("DROP TRIGGER fail_child_delete").unwrap();
        let Response::ChildStop(ChildStopResult::Retired(outcome)) = handle(db, &request(context, "stop-fault"), input).unwrap() else { panic!("retry failed") };
        assert_eq!(outcome.effects.iter().map(|effect| effect.record.name.as_str()).collect::<Vec<_>>(), vec!["nested", "child"]);
    });
}

#[test]
fn combined_flush_failure_rolls_back_mailbox_bootstrap_status_and_history() {
    fixture(|db, context| {
        let root = root_fixture(db);
        let mail = message(db, "sender", &["root"], "work");
        let prepared = output(db, &request(context, "prepare"), &root, true);
        let confirm = Request::ConfirmFlushed { actor: root.clone(), mailbox: receipt(&prepared.mailbox), bootstrap: prepared.bootstrap.map(|offer| offer.receipt), history: None };
        db.conn().execute_batch("CREATE TRIGGER fail_delivery_status BEFORE INSERT ON events WHEN NEW.type='status' AND json_extract(NEW.data,'$.context') LIKE 'deliver:%' BEGIN SELECT RAISE(ABORT,'status failed'); END;").unwrap();
        assert!(handle(db, &request(context, "flush"), confirm.clone()).is_err());
        let row = db.get_instance_full("root").unwrap().unwrap();
        assert_eq!(row.last_event_id, 0);
        assert_eq!(row.name_announced, 0);
        db.conn().execute_batch("DROP TRIGGER fail_delivery_status").unwrap();
        handle(db, &request(context, "flush"), confirm.clone()).unwrap();
        let row = db.get_instance_full("root").unwrap().unwrap();
        assert_eq!(row.last_event_id, mail);
        assert_eq!(row.name_announced, 1);
        handle(db, &request(context, "newer"), Request::Activity { actor: root, activity: Activity::ToolStarted { tool_name: "Read".into(), detail: "new".into(), tool_use_id: None, visible_agentx_shell: false } }).unwrap();
        handle(db, &request(context, "flush"), confirm).unwrap();
        assert_eq!(db.get_instance_full("root").unwrap().unwrap().status_context, "tool:Read");
    });
}

#[test]
fn bootstrap_only_prompt_does_not_issue_mailbox_and_dead_stop_owner_can_be_replaced() {
    fixture(|db, context| {
        let root = root_fixture(db);
        child_fixture(db, &root, "child", "agent", 2.0);
        message(db, "sender", &["root", "child"], "mail remains pending");
        let Response::Output(prepared) = handle(db, &request(context, "bootstrap-only"), Request::PrepareOutput { actor: root.clone(), mailbox: false, bootstrap: true, stop_claim: None }).unwrap() else { panic!("bootstrap only") };
        assert!(matches!(prepared.mailbox, PreparedOutput::Deferred));
        assert!(prepared.bootstrap.is_some());
        let count: i64 = db.conn().query_row("SELECT count(*) FROM kv WHERE key LIKE 'output_receipt:%' OR key LIKE 'claude_mailbox:%'", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 0);
        handle(db, &request(context, "bootstrap-only-flush"), Request::ConfirmFlushed { actor: root.clone(), mailbox: None, bootstrap: prepared.bootstrap.map(|offer| offer.receipt), history: None }).unwrap();
        assert_eq!(db.get_instance_full("root").unwrap().unwrap().last_event_id, 0);
        let ChildStopResult::Acquired { claim: old, .. } = begin_child(db, &request(context, "old-owner"), &root, "agent") else { panic!("old owner") };
        let key: String = db.conn().query_row("SELECT key FROM kv WHERE key LIKE 'subagent_stop_inflight:%'", [], |row| row.get(0)).unwrap();
        let mut old_owner: serde_json::Value = serde_json::from_str(&db.kv_get(&key).unwrap().unwrap()).unwrap();
        old_owner["incarnation"] = json!("previous-observed-incarnation");
        db.kv_set(&key, Some(&old_owner.to_string())).unwrap();
        let ChildStopResult::Acquired { claim: replacement, .. } = begin_child(db, &request(context, "replacement"), &root, "agent") else { panic!("known incarnation mismatch must release old ownership") };
        assert_ne!(old.token, replacement.token);
        assert!(matches!(handle(db, &request(context, "late-old-owner"), Request::FinishChildStop { claim: old, decision: FinishChildStop::TimedOut }), Err(ServiceError::StaleFence)));
        handle(db, &request(context, "replacement-release"), Request::FinishChildStop { claim: replacement, decision: FinishChildStop::Released }).unwrap();
        assert!(db.get_instance_full("child").unwrap().is_some());
    });
}

#[test]
fn task_history_has_own_receipt_and_async_or_unrelated_events_do_not_advance_mailbox() {
    fixture(|db, context| {
        let root = root_fixture(db);
        child_fixture(db, &root, "child", "agent", 2.0);
        let child_event = message(db, "child", &["elsewhere"], "child history");
        let unrelated = message(db, "unrelated", &["elsewhere"], "unrelated");
        let async_request = Request::TaskCompleted { actor: root.clone(), async_launched: true };
        assert!(matches!(handle(db, &request(context, "async"), async_request).unwrap(), Response::TaskOutput(None)));
        let task_request = Request::TaskCompleted { actor: root.clone(), async_launched: false };
        let Response::TaskOutput(Some(task)) = handle(db, &request(context, "history"), task_request.clone()).unwrap() else { panic!("no history") };
        assert_eq!(task.messages.iter().map(|message| message.event.0).collect::<Vec<_>>(), vec![child_event]);
        let history = task.history_receipt.unwrap();
        assert_eq!(history.through.0, child_event);
        assert!(history.through.0 < unrelated);
        assert_eq!(db.get_instance_full("root").unwrap().unwrap().last_event_id, 0);
        let confirm = Request::ConfirmFlushed { actor: root.clone(), mailbox: receipt(&task.mailbox), bootstrap: None, history: Some(history.clone()) };
        handle(db, &request(context, "history-flush"), confirm).unwrap();
        assert_eq!(db.get_instance_full("root").unwrap().unwrap().last_event_id, 0, "history cannot acknowledge an unrelated mailbox frontier");
        let Response::TaskOutput(Some(next)) = handle(db, &request(context, "next-history"), task_request).unwrap() else { panic!("no task response") };
        assert!(next.messages.is_empty());
        assert!(next.history_receipt.is_none());
        let mut forged = history;
        forged.through.0 = unrelated;
        assert!(handle(db, &request(context, "forged-history"), Request::ConfirmFlushed { actor: root, mailbox: None, bootstrap: None, history: Some(forged) }).is_err());
    });
}

#[test]
fn historical_end_cleans_old_session_only_and_unknown_registry_cannot_reap_children() {
    fixture(|db, context| {
        let old = root_fixture(db);
        let child = child_fixture(db, &old, "child", "agent", 2.0);
        db.attach_claude_generation("root", "new-session", "/new", "worker", Some("root")).unwrap();
        let current = Actor { session_id: "new-session".into(), ..old.clone() };
        let input = observation(ObservationKind::Callback, "session");
        assert!(matches!(observed(db, &request(context, "historical"), input), Resolution::Historical { .. }));
        db.kv_set("subagent_stop_inflight:session:old", Some("old-scope")).unwrap();
        let Response::Stopped(outcome) = handle(db, &request(context, "old-end"), Request::SessionEnd { root: old, reason: "clear".into() }).unwrap() else { panic!("end") };
        assert!(outcome.retained);
        assert!(outcome.effects.is_empty());
        assert!(db.kv_get("subagent_stop_inflight:session:old").unwrap().is_none());
        assert_eq!(db.get_instance_full("child").unwrap().unwrap().parent_session_id.as_deref(), Some("new-session"));
        for (id, registry) in [("unknown-registry", None), ("agent-registry", Some(vec![BackgroundTask::MayRunAgent]))] {
            handle(db, &request(context, id), Request::BeginRootStop { actor: current.clone(), background_tasks: registry }).unwrap();
            assert!(db.get_instance_full("child").unwrap().is_some());
        }
        let Response::RootStop { retired_children, .. } =
            handle(db, &request(context, "proven-registry"), Request::BeginRootStop { actor: current, background_tasks: Some(vec![BackgroundTask::Shell, BackgroundTask::Monitor, BackgroundTask::Mcp]) }).unwrap()
        else {
            panic!("root stop")
        };
        assert_eq!(retired_children.effects[0].participant, child.participant);
        assert!(db.get_instance_full("root").unwrap().is_some());
    });
}

#[test]
fn never_ready_pty_defers_but_killed_end_preserves_operator_provenance_and_recovery() {
    fixture(|db, context| {
        let root = root_fixture(db);
        child_fixture(db, &root, "child", "agent", 2.0);
        let mut context = context.clone();
        context.caller.invocation.environment.extend([("AGENTX_PTY_MODE".into(), "1".into()), ("AGENTX_LAUNCHED".into(), "1".into()), ("AGENTX_LAUNCH_BATCH_ID".into(), "never-ready".into())]);
        let end = Request::SessionEnd { root: root.clone(), reason: "error".into() };
        let Response::Stopped(result) = handle(db, &request(&context, "defer"), end.clone()).unwrap() else { panic!("defer") };
        assert!(result.retained);
        assert!(result.effects.is_empty());
        assert!(db.get_instance_full("root").unwrap().is_some());
        db.update_instance_fields("root", json!({"status":"inactive","status_context":"exit:killed","status_detail":"operator","status_time":99}).as_object().unwrap()).unwrap();
        let Response::Stopped(result) = handle(db, &request(&context, "killed"), end.clone()).unwrap() else { panic!("killed") };
        assert!(result.stopped);
        assert_eq!(result.effects.len(), 2);
        assert_eq!(result.effects.last().unwrap().record.status_detail, "operator");
        let after = db.get_last_event_id();
        handle(db, &request(&context, "killed"), end).unwrap();
        assert_eq!(db.get_last_event_id(), after);
        let event: String = db.conn().query_row("SELECT data FROM events WHERE type='life' AND instance='root' AND json_extract(data,'$.action')='stopped' ORDER BY id DESC LIMIT 1", [], |row| row.get(0)).unwrap();
        let event: serde_json::Value = serde_json::from_str(&event).unwrap();
        assert_eq!(event["by"], "operator");
        assert_eq!(event["reason"], "killed");
    });
}
