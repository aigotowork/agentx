//! Internal composition of one managed Codex PTY callback. The adapter owns
//! Provider JSON and stdout; participants and messaging retain their domain
//! authority. See `.docs/proposals/codex-hook-critical-path.md`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::agents::model_request::RequestPhase;
use crate::db::AgentxDb;
use crate::messaging::service::HookOutput;
use crate::shared::{ST_ACTIVE, ST_LISTENING};

use super::{RequestContext, ServiceError, ServiceResult};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct HookEvent {
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub directory: PathBuf,
    pub activity: Activity,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Activity {
    PreTool { tool_name: String, detail: String, written_files: Vec<String> },
    PostTool { tool_name: String },
    Model { phase: RequestPhase, turn_id: String, model: String, error: Option<String> },
    Prompt { triggered: bool, turn_id: Option<String> },
    Stop { interrupted: bool, turn_id: Option<String> },
}

impl Activity {
    pub(crate) fn operation(&self) -> &'static str {
        match self {
            Self::PreTool { .. } => "CodexPreToolUse",
            Self::PostTool { .. } => "CodexPostToolUse",
            Self::Model { phase: RequestPhase::Started, .. } => "CodexPreModelRequest",
            Self::Model { .. } => "CodexPostModelRequest",
            Self::Prompt { .. } => "CodexUserPromptSubmit",
            Self::Stop { interrupted: true, .. } => "CodexInterrupt",
            Self::Stop { interrupted: false, .. } => "CodexStop",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum HookOutcome {
    Ignored,
    Applied,
    NoOutput,
    Output(HookOutput),
}

pub(super) fn run(db: &AgentxDb, context: &RequestContext, event: HookEvent) -> ServiceResult<HookOutcome> {
    let HookEvent { session_id, transcript_path, directory, activity } = event;
    if matches!(&activity, Activity::Model { .. }) && session_id.as_deref().is_none_or(str::is_empty) {
        return Err(ServiceError::InvalidInput("model request session required".into()));
    }
    let Some(actor) = crate::agents::service::resolve_codex_pty(db, context, session_id.as_deref(), transcript_path.as_deref(), &directory)? else {
        return Ok(HookOutcome::Ignored);
    };
    let prepare = || crate::messaging::service::prepare_hook_output(db, context, actor.participant());
    match activity {
        Activity::PreTool { tool_name, detail, written_files } => actor.tool_activity(db, context, tool_name, detail, written_files)?,
        Activity::PostTool { tool_name } => {
            if actor.approval_blocked() {
                actor.status(db, ST_ACTIVE, &format!("approved:{tool_name}"))?;
            }
            return Ok(prepare()?.map_or(HookOutcome::NoOutput, HookOutcome::Output));
        }
        Activity::Model { phase, turn_id, model, error } => {
            let input = crate::agents::model_request::ObserveModelRequest { participant: actor.participant().clone(), session_id: session_id.expect("checked above"), turn_id, model, phase, error };
            actor.model_request(db, context, input)?;
        }
        Activity::Prompt { triggered, turn_id } => {
            actor.observe_recovery_turn(db, context, session_id, turn_id, false);
            if triggered && let Some(prepared) = prepare()? {
                return Ok(HookOutcome::Output(prepared));
            }
            actor.status(db, ST_ACTIVE, "prompt")?;
        }
        Activity::Stop { interrupted, turn_id } => {
            actor.observe_recovery_turn(db, context, session_id, turn_id, !interrupted);
            actor.status(db, ST_LISTENING, if interrupted { "interrupted" } else { "" })?;
            db.wake_after_commit(&actor.participant().canonical_name, &[crate::runtime::notify::WakeKind::Hook]);
        }
    }
    Ok(HookOutcome::Applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::service::{InboxOperation, InboxOutcome, OutputConfirmation, Request as MailRequest, Response as MailResponse};
    use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};
    use serde_json::json;

    #[test]
    fn codex_hook_combines_fenced_updates_and_only_confirms_prepared_mail() {
        let directory = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
        db.save_instance_named("luna", json!({"tool":"codex","created_at":1.0,"session_id":"session","directory":"/project","transcript_path":"/project/session.jsonl"}).as_object().unwrap()).unwrap();
        db.conn().execute("INSERT INTO process_bindings(process_id,session_id,instance_name,updated_at) VALUES ('binding','session','luna',0)", []).unwrap();
        db.conn().execute_batch("CREATE TEMP TRIGGER unchanged_position BEFORE UPDATE OF directory, session_id, transcript_path ON instances BEGIN SELECT RAISE(ABORT,'unchanged position write'); END;").unwrap();
        let pid = std::process::id();
        let mut context = RequestContext {
            caller: CallerContext { request_id: RequestId("first".into()), invocation: InvocationContext { process_binding_id: Some("binding".into()), ..Default::default() }, deadline_unix_ms: None, expected_database_generation: None },
            process: CallerProcess { pid, incarnation: crate::sys::process::identity(pid) },
            database_generation: "generation".into(),
            daemon_instance: "daemon".into(),
        };
        let event = |activity| HookEvent { session_id: Some("session".into()), transcript_path: Some("/project/session.jsonl".into()), directory: "/project".into(), activity };
        let mut wrong = event(Activity::PostTool { tool_name: "Bash".into() });
        wrong.session_id = Some("other".into());
        assert!(matches!(run(&db, &context, wrong), Err(ServiceError::StaleFence)));
        let post_tool = || event(Activity::PostTool { tool_name: "Bash".into() });
        assert!(matches!(run(&db, &context, post_tool()).unwrap(), HookOutcome::NoOutput));
        let receipts = || db.conn().query_row("SELECT count(*) FROM kv WHERE key GLOB 'output_receipt:*'", [], |row| row.get::<_, i64>(0)).unwrap();
        assert_eq!(receipts(), 0);
        assert!(matches!(run(&db, &context, event(Activity::PreTool { tool_name: "Bash".into(), detail: "echo fixture".into(), written_files: Vec::new() })).unwrap(), HookOutcome::Applied));
        let enqueue = |text: &str| db.insert_event_row("message", "sender", &json!({"from":"sender","scope":"mentions","mentions":["luna"],"text":text}), None).unwrap();
        let first_batch: Vec<_> = (1..=5).map(|index| enqueue(&format!("ready {index}"))).collect();
        context.caller.request_id = RequestId("second".into());
        let HookOutcome::Output(HookOutput { confirmation, formatted_context }) = run(&db, &context, post_tool()).unwrap() else { panic!("expected one prepared batch") };
        assert!(matches!(confirmation, OutputConfirmation::Unmanaged(_)));
        for id in &first_batch {
            assert_eq!(formatted_context.matches(&format!("[new message #{id}]")).count(), 1);
        }
        let confirm = |context: &RequestContext, receipt| {
            let response = crate::messaging::service::handle(&db, context, MailRequest::Inbox(InboxOperation::ConfirmOutputAndStatus { receipt, announce: false })).unwrap();
            assert!(matches!(response, MailResponse::Inbox(InboxOutcome::Done(Ok(())))));
        };
        let mut contender = context.clone();
        // Reproduce the five callbacks overlapping while the first holds its output.
        for index in 0..4 {
            contender.caller.request_id = RequestId(format!("contender-{index}"));
            assert!(matches!(run(&db, &contender, post_tool()).unwrap(), HookOutcome::NoOutput));
        }
        let participant = crate::agents::attachments::ParticipantRef { canonical_name: "luna".into(), incarnation_bits: 1.0_f64.to_bits() };
        let prepare = |consumer| crate::messaging::service::handle(&db, &contender, MailRequest::Inbox(InboxOperation::PrepareOutput { participant: participant.clone(), consumer })).unwrap();
        use crate::messaging::{inbox::InboxConsumer, service::PreparedOutput};
        for consumer in [InboxConsumer::Hook, InboxConsumer::ExplicitListen] {
            assert!(matches!(prepare(consumer), MailResponse::Inbox(InboxOutcome::Output(Ok(PreparedOutput::Deferred)))));
        }
        assert_eq!(receipts(), 1);
        let cursor = || db.get_instance_status("luna").unwrap().unwrap().last_event_id;
        assert_eq!(cursor(), 0);
        let next = enqueue("arrived while first batch was held");
        confirm(&context, confirmation);
        assert_eq!(cursor(), *first_batch.last().unwrap());
        let HookOutcome::Output(output) = run(&db, &contender, post_tool()).unwrap() else { panic!("expected the next message") };
        assert!(output.formatted_context.contains(&format!("[new message #{next}]")));
        assert!(first_batch.iter().all(|id| !output.formatted_context.contains(&format!("[new message #{id}]"))));
        confirm(&contender, output.confirmation);
        assert_eq!(cursor(), next);
        assert!(matches!(run(&db, &contender, post_tool()).unwrap(), HookOutcome::NoOutput));
    }
}
