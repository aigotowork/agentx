//! Actual Claude host observations and stdout over a disposable native IPC
//! daemon. Setup storage closes before the real host adapter connects.
use super::*;
use crate::agents::attachments::{AttachmentIntent, AttachmentOwnerId, AttachmentOwnerProcess, AttachmentRequest, AttachmentStore, ExecutionId, ProviderSessionKey, SqliteAttachmentStore};
use crate::db::{AgentxDb, InstanceRow};

struct Fixture {
    hook: HookClient,
    _daemon: crate::daemon::test_support::TestDaemon,
    _directory: tempfile::TempDir,
}

fn fixture(managed: bool, seed: impl FnOnce(&AgentxDb)) -> Fixture {
    let directory = tempfile::Builder::new().prefix("agentx-claude-host-").tempdir().unwrap();
    crate::paths::test_roots::register(directory.path());
    let scope = directory.path().join(".agentx");
    let cwd = directory.path().join("origin");
    std::fs::create_dir_all(&scope).unwrap();
    std::fs::create_dir_all(&cwd).unwrap();
    let database = scope.join("agentx.db");
    let db = AgentxDb::open_at(&database).unwrap();
    db.save_instance_named("root", json!({"tool":"claude","session_id":"session","created_at":1.0,"status":"listening","status_context":"start","name_announced":0,"last_event_id":0,"directory":cwd,"subagent_timeout":1}).as_object().unwrap())
        .unwrap();
    db.set_session_binding("session", "root").unwrap();
    db.set_process_binding("worker", "session", "root").unwrap();
    db.mark_claude_session_validated("session", "root").unwrap();
    seed(&db);
    let attachment = if managed {
        let mut store = SqliteAttachmentStore::for_owner_process(&db, AttachmentOwnerProcess { pid: std::process::id(), process_identity: crate::sys::process::identity(std::process::id()) });
        let participant = store.participant("root").unwrap();
        Some(
            store
                .attach(AttachmentRequest {
                    execution: ExecutionId("claude-host".into()),
                    owner: AttachmentOwnerId("claude-host-owner".into()),
                    session: ProviderSessionKey { provider: "claude".into(), scope: scope.to_string_lossy().into_owned(), session_id: "session".into() },
                    intent: AttachmentIntent::Create { provisional: participant },
                })
                .unwrap(),
        )
    } else {
        None
    };
    drop(db);
    let daemon = crate::daemon::test_support::TestDaemon::start(&scope, &database);
    let environment = std::collections::HashMap::from([
        ("AGENTX_DIR".into(), scope.to_string_lossy().into_owned()),
        ("HOME".into(), directory.path().to_string_lossy().into_owned()),
        ("AGENTX_TEST_ROOT".into(), directory.path().to_string_lossy().into_owned()),
        ("AGENTX_PROCESS_ID".into(), "worker".into()),
        ("AGENTX_TOOL".into(), "claude".into()),
        ("AGENTX_LAUNCHED".into(), "1".into()),
        ("AGENTX_PTY_MODE".into(), "1".into()),
    ]);
    let context = AgentxContext::from_env(&environment, cwd).with_tty(false, false);
    let mut hook = HookClient::connect(&context, false).unwrap();
    if let Some(attachment) = attachment {
        let mut invocation = hook.client.invocation().clone();
        invocation.activation_target = Some(attachment.target);
        invocation.provider_session = Some(attachment.session);
        hook.client = hook.client.with_invocation(invocation);
    }
    Fixture { hook, _daemon: daemon, _directory: directory }
}

fn record(hook: &HookClient, name: &str) -> Option<InstanceRow> {
    match hook.participant(ParticipantRequest::Record { name: name.into() }).unwrap() {
        ParticipantResponse::Record(record) => record,
        _ => panic!("unexpected native record"),
    }
}
fn payload(value: Value) -> HookPayload {
    HookPayload::from_claude(value)
}
fn mail(db: &AgentxDb, from: &str, to: &str, text: &str) -> i64 {
    db.log_event("message", from, &json!({"from":from,"text":text,"scope":"mentions","mentions":[to]})).unwrap()
}
fn child(db: &AgentxDb) {
    db.save_instance_named(
        "child",
        json!({"tool":"claude","created_at":2.0,"parent_name":"root","parent_session_id":"session","agent_id":"agent-one","status":"inactive","status_context":"subagent:dormant","name_announced":0,"last_event_id":0,"directory":"/child"})
            .as_object()
            .unwrap(),
    )
    .unwrap();
}
struct FlushFailure;
impl Write for FlushFailure {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("fixture stdout flush failed"))
    }
}

#[test]
fn actual_pty_prompt_flush_failure_preserves_mail_and_bootstrap_then_retry_confirms_both() {
    let event = std::cell::Cell::new(0);
    let fixture = fixture(false, |db| event.set(mail(db, "sender", "root", "exact frozen body")));
    let hook = &fixture.hook;
    let prompt = payload(json!({"session_id":"session","prompt":"<agentx>"}));
    let output = route(hook, super::super::HOOK_USERPROMPTSUBMIT, &prompt).unwrap();
    assert!(flush(hook, output, &mut FlushFailure).is_err());
    let row = record(hook, "root").unwrap();
    assert_eq!((row.last_event_id, row.name_announced), (0, 0));
    let output = route(hook, super::super::HOOK_USERPROMPTSUBMIT, &prompt).unwrap();
    let mut bytes = Vec::new();
    flush(hook, output, &mut bytes).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(value["systemMessage"].as_str().unwrap().contains("exact frozen body"));
    assert!(value["hookSpecificOutput"]["additionalContext"].as_str().unwrap().contains("<agentx>"));
    let row = record(hook, "root").unwrap();
    assert_eq!((row.last_event_id, row.name_announced), (event.get(), 1));
    assert_eq!(row.status, "active");
    assert!(row.status_context.starts_with("deliver:"));
}

#[test]
fn managed_failed_stdout_remains_owned_and_another_prompt_cannot_reissue() {
    let fixture = fixture(true, |db| {
        db.update_instance_fields("root", json!({"name_announced":1}).as_object().unwrap()).unwrap();
        mail(db, "sender", "root", "managed uncertain body");
    });
    let prompt = payload(json!({"session_id":"session","prompt":"<agentx>"}));
    let first = route(&fixture.hook, super::super::HOOK_USERPROMPTSUBMIT, &prompt).unwrap();
    assert!(matches!(first.mailbox, Some(OutputConfirmation::Claimed(_))));
    assert!(flush(&fixture.hook, first, &mut FlushFailure).is_err());
    let second = route(&fixture.hook, super::super::HOOK_USERPROMPTSUBMIT, &prompt).unwrap();
    assert!(second.mailbox.is_none());
    assert!(!second.value.as_ref().unwrap().to_string().contains("managed uncertain body"));
    assert_eq!(record(&fixture.hook, "root").unwrap().last_event_id, 0);
}

#[test]
fn nonpty_prompt_recovers_bootstrap_without_claiming_pending_mail() {
    let mut fixture = fixture(false, |db| {
        mail(db, "sender", "root", "not for nonpty prompt");
    });
    fixture.hook.context.is_pty_mode = false;
    let mut invocation = fixture.hook.client.invocation().clone();
    invocation.environment.remove("AGENTX_PTY_MODE");
    fixture.hook.client = fixture.hook.client.with_invocation(invocation);
    let output = route(&fixture.hook, super::super::HOOK_USERPROMPTSUBMIT, &payload(json!({"session_id":"session","prompt":"ordinary prompt"}))).unwrap();
    assert!(output.mailbox.is_none());
    assert!(output.bootstrap.is_some());
    assert!(!output.value.as_ref().unwrap().to_string().contains("not for nonpty prompt"));
    flush(&fixture.hook, output, &mut Vec::new()).unwrap();
    let row = record(&fixture.hook, "root").unwrap();
    assert_eq!((row.last_event_id, row.name_announced), (0, 1));
}

#[test]
fn actual_child_shell_is_correlated_and_terminal_failure_revokes_its_capability() {
    let fixture = fixture(false, child);
    let pre = payload(json!({"session_id":"session","agent_id":"agent-one","tool_name":"Bash","tool_use_id":"shell-one","tool_input":{"command":"agentx send @root -- child result"}}));
    let output = route(&fixture.hook, super::super::HOOK_PRE, &pre).unwrap();
    let command = output.value.as_ref().unwrap()["hookSpecificOutput"]["updatedInput"]["command"].as_str().unwrap();
    assert!(command.contains(super::super::actor::ENV_VAR));
    assert!(command.ends_with("agentx send @root -- child result"));
    assert_eq!(record(&fixture.hook, "root").unwrap().status_context, "start");
    assert_eq!(record(&fixture.hook, "child").unwrap().status_context, "tool:Bash");
    let failed = payload(json!({"session_id":"session","agent_id":"agent-one","tool_name":"Bash","tool_use_id":"shell-one","error":"shell failed"}));
    route(&fixture.hook, super::super::HOOK_POST_FAILURE, &failed).unwrap();
    assert_eq!(record(&fixture.hook, "child").unwrap().status_context, "tool_failed:Bash");
    // Reissuing the same tool-use after terminal revocation produces a new
    // token, so a shell that retained its old environment cannot regain it.
    let next = route(&fixture.hook, super::super::HOOK_PRE, &pre).unwrap();
    assert_ne!(output.value, next.value);
}

#[test]
fn unknown_child_shell_is_denied_and_bare_wake_cannot_mutate_root() {
    let fixture = fixture(false, |_| {});
    let output = route(&fixture.hook, super::super::HOOK_PRE, &payload(json!({"session_id":"session","agent_id":"unknown","tool_name":"Bash","tool_input":{"command":"agentx start"}}))).unwrap();
    assert_eq!(output.value.unwrap()["hookSpecificOutput"]["permissionDecision"], "deny");
    let wake = route(&fixture.hook, super::super::HOOK_USERPROMPTSUBMIT, &payload(json!({"session_id":"session","agent_id":"unknown","prompt":"<agentx>"}))).unwrap();
    assert_eq!(wake.value.unwrap()["decision"], "block");
    let row = record(&fixture.hook, "root").unwrap();
    assert_eq!((row.last_event_id, row.name_announced), (0, 0));
    assert_eq!(row.status_context, "start");
}

#[test]
fn actual_subagent_stop_combines_direct_mail_bootstrap_and_postflush_child_receipt() {
    let event = std::cell::Cell::new(0);
    let fixture = fixture(false, |db| {
        child(db);
        event.set(mail(db, "sender", "child", "direct child work"));
    });
    let output = route(&fixture.hook, super::super::HOOK_SUBAGENT_STOP, &payload(json!({"session_id":"session","agent_id":"agent-one","prompt_id":"stop-one","agent_transcript_path":"/child/trace.jsonl"}))).unwrap();
    assert_eq!(output.value.as_ref().unwrap()["decision"], "block");
    let reason = output.value.as_ref().unwrap()["reason"].as_str().unwrap();
    assert!(reason.contains("direct child work"));
    assert!(reason.contains("child"));
    let before = record(&fixture.hook, "child").unwrap();
    assert_eq!((before.last_event_id, before.name_announced), (0, 0));
    flush(&fixture.hook, output, &mut Vec::new()).unwrap();
    let after = record(&fixture.hook, "child").unwrap();
    assert_eq!((after.last_event_id, after.name_announced), (event.get(), 1));
    assert_eq!(after.tcp_mode, 0);
    assert_eq!(record(&fixture.hook, "root").unwrap().last_event_id, 0);
}

#[test]
fn actual_task_output_flush_controls_history_frontier_without_acknowledging_unrelated_mail() {
    let fixture = fixture(false, |db| {
        child(db);
        mail(db, "child", "elsewhere", "task child history");
        mail(db, "unrelated", "elsewhere", "unrelated history");
    });
    let task = payload(json!({"session_id":"session","tool_name":"Task","tool_response":{"status":"completed"}}));
    let failed = route(&fixture.hook, super::super::HOOK_POST, &task).unwrap();
    assert!(flush(&fixture.hook, failed, &mut FlushFailure).is_err());
    let retry = route(&fixture.hook, super::super::HOOK_POST, &task).unwrap();
    let text = retry.value.as_ref().unwrap().to_string();
    assert!(text.contains("task child history"));
    assert!(!text.contains("unrelated history"));
    flush(&fixture.hook, retry, &mut Vec::new()).unwrap();
    assert_eq!(record(&fixture.hook, "root").unwrap().last_event_id, 0);
    let next = route(&fixture.hook, super::super::HOOK_POST, &task).unwrap();
    assert!(next.value.is_none());
    let asynchronous = payload(json!({"session_id":"session","tool_name":"Agent","tool_response":{"status":"async_launched"}}));
    assert!(route(&fixture.hook, super::super::HOOK_POST, &asynchronous).unwrap().value.is_none());
}

#[test]
fn root_stop_preserves_unknown_children_and_transport_loss_never_opens_storage() {
    let fixture = fixture(false, child);
    let output = route(&fixture.hook, super::super::HOOK_POLL, &payload(json!({"session_id":"session"}))).unwrap();
    assert!(output.value.is_none());
    assert!(record(&fixture.hook, "child").is_some());
    assert_eq!(record(&fixture.hook, "root").unwrap().status, "listening");
    let Fixture { hook, _daemon: daemon, _directory: directory } = fixture;
    drop(daemon);
    let path = directory.path().join(".agentx/agentx.db");
    std::fs::remove_file(&path).unwrap();
    assert!(route(&hook, super::super::HOOK_PRE, &payload(json!({"session_id":"session","tool_name":"Bash","tool_input":{"command":"pwd"}}))).is_err());
    assert!(!path.exists());
}
