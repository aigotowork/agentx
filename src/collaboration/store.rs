//! Daemon-only composition. Work/receipt and existing native message admission
//! share one outer write scope, including deferred wake/event rollback.
use super::model::{PendingMessage, invalid, valid_id};
use super::service::*;
use crate::{
    agents::{attachments::ParticipantRef, service as participants},
    db::{AgentxDb, StoredWorkCommand},
    service::{RequestContext, ServiceError, ServiceResult},
};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, thiserror::Error)]
enum Fault {
    #[error("work rejected: {0:?}")]
    Rejected(Failure),
    #[error(transparent)]
    Service(#[from] ServiceError),
}
impl From<Failure> for Fault {
    fn from(value: Failure) -> Self {
        Self::Rejected(value)
    }
}
fn internal(error: impl std::fmt::Display) -> Fault {
    Fault::Service(ServiceError::Internal(error.to_string()))
}
type Outcome<T> = Result<T, Fault>;

pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    let result = match request {
        Request::Read { generation, project, read } => read_work(db, context, &generation, &project, read).map(Response::Read),
        Request::Apply(call) => db.with_write_scope(|| apply(db, context, call).map_err(anyhow::Error::new)).map_err(|e| e.downcast_ref::<Fault>().cloned().unwrap_or_else(|| internal(e))).map(Response::Applied),
    };
    match result {
        Ok(value) => Ok(value),
        Err(Fault::Rejected(reason)) => Ok(Response::Rejected(reason)),
        Err(Fault::Service(error)) => Err(error),
    }
}
fn project_valid(project: &str) -> Result<(), Failure> {
    use std::path::{Component, Path};
    if project.is_empty() || project.len() > 4096 || !Path::new(project).is_absolute() || Path::new(project).components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
        return Err(invalid("project", "canonical absolute project path required"));
    }
    Ok(())
}
fn scope(db: &AgentxDb) -> Outcome<String> {
    db.path().parent().and_then(|p| p.to_str()).map(str::to_owned).ok_or_else(|| internal("scope is not representable"))
}
fn fence(context: &RequestContext, generation: &str) -> Result<(), Failure> {
    if generation != context.database_generation { Err(Failure::StaleReference) } else { Ok(()) }
}
fn reference_valid(db: &AgentxDb, context: &RequestContext, project: &str, reference: &WorkRef) -> Outcome<()> {
    if reference.scope != scope(db)? || reference.generation != context.database_generation || reference.project != project || !valid_id(&reference.id) {
        return Err(Failure::StaleReference.into());
    }
    Ok(())
}
fn load(db: &AgentxDb, context: &RequestContext, project: &str, reference: &WorkRef) -> Outcome<Work> {
    reference_valid(db, context, project, reference)?;
    let payload = db.collaboration_work(&context.database_generation, &reference.id).map_err(internal)?.ok_or(Failure::NotFound)?;
    let work: Work = serde_json::from_str(&payload).map_err(internal)?;
    if &work.reference != reference {
        return Err(Failure::StaleReference.into());
    }
    Ok(work)
}
fn participant_directory_matcher(project: &str) -> impl Fn(&participants::ParticipantView) -> bool + '_ {
    #[cfg(windows)]
    let verified_child_spelling = std::cell::OnceCell::new();
    move |record| {
        if record.directory == project {
            return true;
        }
        #[cfg(windows)]
        {
            use std::path::Path;
            if record.origin_device_id.as_ref().is_some_and(|id| !id.is_empty()) {
                return false;
            }
            // A local Provider can report the ordinary cwd given to its child
            // process. Verify only that spelling of the selected project, at
            // most once per read, and never resolve a record's arbitrary path.
            let project = Path::new(project);
            let child_directory = crate::shared::platform::child_process_path(project);
            child_directory != project && child_directory.is_absolute() && Path::new(&record.directory) == child_directory && *verified_child_spelling.get_or_init(|| child_directory.canonicalize().is_ok_and(|canonical| canonical == project))
        }
        #[cfg(not(windows))]
        false
    }
}

fn participant_record(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef, project: &str) -> Outcome<participants::ParticipantView> {
    let record = match participants::handle(db, context, participants::Request::Record { name: participant.canonical_name.clone() })? {
        participants::Response::Record(Some(record)) => record,
        _ => return Err(Failure::StaleReference.into()),
    };
    if record.name != participant.canonical_name || record.created_at.to_bits() != participant.incarnation_bits {
        return Err(Failure::StaleReference.into());
    }
    if record.origin_device_id.as_ref().is_some_and(|id| !id.is_empty()) || matches!(record.status.as_str(), "inactive" | "stopped" | "killed") || !participant_directory_matcher(project)(&record) {
        return Err(Failure::Forbidden.into());
    }
    Ok(record)
}
fn page_valid(page: &Page) -> Result<(), Failure> {
    if page.limit == 0 || page.limit > MAX_PAGE || page.after.as_ref().is_some_and(|s| s.is_empty() || s.len() > 128) {
        return Err(invalid("page", "bounded nonzero page required"));
    }
    Ok(())
}
fn read_work(db: &AgentxDb, context: &RequestContext, generation: &str, project: &str, read: Read) -> Outcome<ReadValue> {
    fence(context, generation)?;
    project_valid(project)?;
    match read {
        Read::Thread { work, page } => {
            load(db, context, project, &work)?;
            if page.limit == 0 || page.limit > MAX_PAGE || page.after.is_some_and(|id| id < 0) {
                return Err(invalid("page", "nonnegative bounded thread page required").into());
            }
            // Work IDs are native UUID/ASCII IDs validated above. No HTTP
            // filter, SQL, thread selector or raw event JSON crosses this map.
            let query = crate::events::service::EventQuery { predicate: Some(format!("type='message' AND msg_thread='work-{}'", work.id)), after: page.after, limit: page.limit as usize, oldest_first: true, ..Default::default() };
            let result = crate::events::store::handle(db, context, crate::events::service::Request::Query(query))?;
            let crate::events::service::Response::Page(events) = result else {
                return Err(internal("unexpected thread read result"));
            };
            let mut items = Vec::new();
            let mut bytes = 0;
            let mut next = None;
            for event in &events.events {
                let data = &event["data"];
                let message = ThreadMessage {
                    event: event["id"].as_i64().ok_or_else(|| internal("missing event identity"))?,
                    at: event["ts"].as_str().map(str::to_owned),
                    author: data["from"].as_str().unwrap_or("unknown").into(),
                    text: data["text"].as_str().unwrap_or("").into(),
                    reply_to: data["reply_to"].as_str().map(str::to_owned),
                    intent: data["intent"].as_str().map(str::to_owned),
                };
                let size = serde_json::to_vec(&message).map_err(internal)?.len();
                if bytes + size > MAX_RECORD_BYTES {
                    if items.is_empty() {
                        return Err(Failure::Capacity.into());
                    }
                    next = items.last().map(|m: &ThreadMessage| m.event);
                    break;
                }
                bytes += size;
                items.push(message);
            }
            if next.is_none() && events.has_more {
                next = items.last().map(|m| m.event);
            }
            Ok(ReadValue::Thread { items, next })
        }
        Read::Detail(reference) => {
            let work = load(db, context, project, &reference)?;
            Ok(ReadValue::Detail(Box::new(WorkDetail { eligibility: work.eligibility(), work })))
        }
        Read::List(page) => {
            page_valid(&page)?;
            let rows = db.collaboration_list(generation, project, page.after.as_deref().unwrap_or(""), page.limit + 1).map_err(internal)?;
            let mut items = rows.into_iter().map(|row| serde_json::from_str::<Work>(&row).map(|w| w.summary()).map_err(internal)).collect::<Outcome<Vec<_>>>()?;
            let more = items.len() > page.limit as usize;
            items.truncate(page.limit as usize);
            let next = if more { items.last().map(|w| w.reference.id.clone()) } else { None };
            Ok(ReadValue::List { items, next })
        }
        Read::Candidates(page) => {
            page_valid(&page)?;
            let records = match participants::handle(db, context, participants::Request::List)? {
                participants::Response::Records(rows) => rows,
                _ => return Err(internal("unexpected participant list")),
            };
            let in_project = participant_directory_matcher(project);
            let mut items: Vec<_> = records
                .into_iter()
                .filter(|record| page.after.as_ref().is_none_or(|after| &record.name > after) && in_project(record))
                .map(|record| {
                    let participant = ParticipantRef { canonical_name: record.name.clone(), incarnation_bits: record.created_at.to_bits() };
                    let unavailable = if record.origin_device_id.as_ref().is_some_and(|s| !s.is_empty()) {
                        Some("Remote participant".into())
                    } else if matches!(record.status.as_str(), "inactive" | "stopped" | "killed") {
                        Some("Participant is inactive".into())
                    } else {
                        None
                    };
                    Candidate { participant, label: record.name, tool: record.tool, runtime_status: record.status, unavailable }
                })
                .collect();
            items.sort_by(|a, b| a.participant.canonical_name.cmp(&b.participant.canonical_name));
            let more = items.len() > page.limit as usize;
            items.truncate(page.limit as usize);
            let next = if more { items.last().map(|c| c.participant.canonical_name.clone()) } else { None };
            Ok(ReadValue::Candidates { items, next })
        }
        Read::Command { command_id } => {
            if !valid_id(&command_id) {
                return Err(invalid("command_id", "invalid command ID").into());
            }
            let stored = db.collaboration_command(generation, &command_id).map_err(internal)?;
            let change = stored.filter(|s| s.project == project).map(|s| serde_json::from_str::<Change>(&s.outcome).map_err(internal)).transpose()?;
            Ok(ReadValue::Command(change))
        }
    }
}
fn apply(db: &AgentxDb, context: &RequestContext, call: CommandCall) -> Outcome<Change> {
    fence(context, &call.generation)?;
    project_valid(&call.project)?;
    if !valid_id(&call.command_id) {
        return Err(invalid("command_id", "1–96 ASCII letters/digits/hyphen/underscore required").into());
    }
    let resolved_actor = if matches!(call.command, Command::Report { .. }) { Some(participants::resolve_actor(db, context)?) } else { None };
    let actor = resolved_actor.as_ref().map(|a| serde_json::to_string(&a.participant).map_err(internal)).transpose()?.unwrap_or_else(|| "operator".into());
    let intent_bytes = serde_json::to_vec(&call).map_err(internal)?;
    if intent_bytes.len() > MAX_RECORD_BYTES {
        return Err(Failure::Capacity.into());
    }
    let intent: String = Sha256::digest(&intent_bytes).iter().map(|b| format!("{b:02x}")).collect();
    // Durable replay is deliberately before current work revision/assignment.
    if let Some(stored) = db.collaboration_command(&call.generation, &call.command_id).map_err(internal)? {
        if stored.project != call.project || stored.actor != actor || stored.intent != intent {
            return Err(Failure::CommandConflict.into());
        }
        let mut change: Change = serde_json::from_str(&stored.outcome).map_err(internal)?;
        change.replayed = true;
        return Ok(change);
    }
    let at = chrono::Utc::now().to_rfc3339();
    let (mut work, effect, pending) = match call.command.clone() {
        Command::Operator(OperatorCommand::CreateWork { brief, implementer, checker }) => {
            if call.work.is_some() || call.expected_revision.is_some() {
                return Err(invalid("work", "create has no existing reference/revision").into());
            }
            if db.collaboration_count(&call.generation).map_err(internal)? >= MAX_WORKS {
                return Err(Failure::Capacity.into());
            }
            participant_record(db, context, &implementer, &call.project)?;
            participant_record(db, context, &checker, &call.project)?;
            let reference = WorkRef { scope: scope(db)?, project: call.project.clone(), generation: call.generation.clone(), id: uuid::Uuid::new_v4().to_string() };
            (Work::create(reference, brief, implementer, checker, at.clone())?, Effect::Saved, None)
        }
        command => {
            let reference = call.work.as_ref().ok_or_else(|| invalid("work", "work reference required"))?;
            let mut work = load(db, context, &call.project, reference)?;
            if call.expected_revision != Some(work.revision) {
                return Err(Failure::RevisionConflict { current_revision: work.revision }.into());
            }
            let (effect, pending) = match command {
                Command::Operator(operator) => {
                    if let OperatorCommand::BindParticipant { participant, .. } = &operator {
                        participant_record(db, context, participant, &call.project)?;
                    }
                    work.apply_operator(operator, &call.command_id, &at)?
                }
                Command::Report { assignment_id, brief_revision, report } => {
                    let actor = resolved_actor.as_ref().ok_or(Failure::Forbidden)?;
                    participant_record(db, context, &actor.participant, &call.project)?;
                    if let AgentReport::Result(result) = &report {
                        for artifact in &result.artifacts {
                            if let Artifact::Message { event } = artifact
                                && !db.collaboration_message_reference(*event, &work.reference.id).map_err(internal)?
                            {
                                return Err(invalid("artifact.event", "message must belong to this work thread").into());
                            }
                        }
                    }
                    work.report(assignment_id, brief_revision, &actor.participant, report, &call.command_id, &at)?;
                    (Effect::Reported, None)
                }
            };
            (work, effect, pending)
        }
    };
    if db.collaboration_command_count(&call.generation, &work.reference.id).map_err(internal)? >= MAX_HISTORY {
        return Err(Failure::Capacity.into());
    }
    let mut messages = Vec::new();
    if let Some(pending) = pending {
        let receipt = admit_message(db, context, &call.command_id, &work, &pending)?;
        messages.push(receipt.clone());
        let assignment_ids = pending.roles.iter().map(|r| work.assignment(r).id.clone()).collect();
        let result_id = if matches!(pending.kind, MessageKind::CheckRequest | MessageKind::Changes) { work.results.last().map(|r| r.id.clone()) } else { None };
        work.messages.push(WorkMessage { id: pending.id, kind: pending.kind, text: pending.text, receipt, at, assignment_ids, brief_revision: work.brief().revision, result_id });
    }
    let payload = serde_json::to_string(&work).map_err(internal)?;
    if work.history_len() > MAX_HISTORY || payload.len() > MAX_RECORD_BYTES {
        return Err(Failure::Capacity.into());
    }
    let change = Change { command_id: call.command_id.clone(), reference: work.reference.clone(), revision: work.revision, effect, messages, replayed: false };
    db.collaboration_save(&call.generation, &work.reference.id, &call.project, work.revision, &payload).map_err(internal)?;
    db.collaboration_retain(&call.generation, &call.command_id, &work.reference.id, &StoredWorkCommand { project: call.project, actor, intent, outcome: serde_json::to_string(&change).map_err(internal)? }).map_err(internal)?;
    Ok(change)
}

/// All targets are exact current project participants checked within the outer
/// transaction. Native Messages remains sole routing/admission/wake authority.
fn admit_message(db: &AgentxDb, context: &RequestContext, command_id: &str, work: &Work, pending: &PendingMessage) -> Outcome<MessageReceipt> {
    use crate::messaging::{
        routing::{MessageEnvelope, MessageIntent},
        service as messages,
    };
    let mut targets = Vec::new();
    for role in &pending.roles {
        let participant = work.assignment(role).participant.clone();
        participant_record(db, context, &participant, &work.reference.project)?;
        targets.push(participant);
    }
    let mut message_context = context.clone();
    message_context.caller.request_id.0 = format!("work-message-{command_id}");
    message_context.caller.invocation = crate::service::InvocationContext { cwd: work.reference.project.clone().into(), ..Default::default() };
    let bindings = pending.roles.iter().map(|role| work.assignment(role).clone()).collect::<Vec<_>>();
    let scope = serde_json::to_string(&work.reference).map_err(internal)?;
    let binding_json = serde_json::to_string(&bindings).map_err(internal)?;
    let text = format!(
        "AgentX work {scope}\nAssignments: {binding_json}\n{}\nReport through: agentx work report --scope '{}' --project '{}' --work {} --generation {} --revision {} --assignment ASSIGNMENT_ID --brief {} --command-id NEW_UUID --payload AGENT_REPORT_JSON\nRead current revision with agentx work show before reporting. Report only your exact assignment; message admission does not establish completion.",
        pending.text,
        work.reference.scope.replace('\'', "'\\''"),
        work.reference.project.replace('\'', "'\\''"),
        work.reference.id,
        work.reference.generation,
        work.revision,
        work.brief().revision
    );
    let text = format!("{text}\n{}", REPORT_GUIDANCE);
    let submission = messages::Submission {
        recipients: targets.iter().map(|p| p.canonical_name.clone()).collect(),
        text,
        envelope: MessageEnvelope { thread: Some(format!("work-{}", work.reference.id)), intent: Some(if matches!(pending.kind, MessageKind::Note) { MessageIntent::Inform } else { MessageIntent::Request }), ..Default::default() },
        external_sender: Some("workbench".into()),
        broadcast_confirmed: false,
        inline_bundle: None,
    };
    let accepted = match messages::handle(db, &message_context, messages::Request::Submit(submission))? {
        messages::Response::Accepted(accepted) => accepted,
        _ => return Err(internal("unexpected message admission")),
    };
    let mut actual = accepted.recipients;
    actual.sort();
    let mut expected = targets.iter().map(|p| p.canonical_name.clone()).collect::<Vec<_>>();
    expected.sort();
    if actual != expected {
        return Err(Failure::Forbidden.into());
    }
    Ok(MessageReceipt { event: accepted.event.0, recipients: targets })
}
