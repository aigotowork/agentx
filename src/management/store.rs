//! Daemon-only management projections and one-shot host operation receipts.
#[cfg(test)]
mod tests;

use super::contract::*;
use crate::agents::{attachments::ParticipantRef, labels};
use crate::db::{AgentxDb, InstanceRow};
use crate::service::{CallerProcess, RequestContext, ServiceError, ServiceResult};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const PREFIX: &str = "management:operation:";
const MAX_RESULT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Record {
    operation: Operation,
    command: Command,
    ticket: String,
    daemon: String,
    worker: Option<(u32, String)>,
    accepted_at: i64,
}
fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn invalid(reason: &str) -> ServiceError {
    ServiceError::InvalidInput(reason.into())
}
fn key(generation: &str, id: &str) -> String {
    format!("{PREFIX}{generation}:{id}")
}
fn load(db: &AgentxDb, context: &RequestContext, id: &str) -> ServiceResult<Record> {
    db.kv_get(&key(&context.database_generation, id)).map_err(internal)?.ok_or_else(|| ServiceError::NotFound("operation".into())).and_then(|value| serde_json::from_str(&value).map_err(internal))
}
pub(crate) fn operation_record(db: &AgentxDb, context: &RequestContext, id: &str) -> ServiceResult<Record> {
    load(db, context, id)
}
fn save(db: &AgentxDb, record: &Record) -> ServiceResult<()> {
    let value = serde_json::to_string(record).map_err(internal)?;
    db.kv_set(&key(&record.operation.generation, &record.operation.request_id), Some(&value)).map_err(internal)
}
fn transaction<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| action().map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}
fn active(record: &Record) -> bool {
    matches!(record.operation.status, OperationStatus::Accepted | OperationStatus::Running)
}
fn reconcile_view(context: &RequestContext, record: &mut Record) -> bool {
    if !active(record) {
        return false;
    }
    // A live exact worker may finish across a daemon restart. Never steal or replay it.
    let gone = record.worker.as_ref().is_some_and(|(pid, incarnation)| crate::sys::process::has_exited(*pid) || crate::sys::process::identity(*pid).is_some_and(|current| current != *incarnation));
    let unclaimed = record.worker.is_none() && (record.daemon != context.daemon_instance || crate::shared::time::now_epoch_i64().saturating_sub(record.accepted_at) > 30);
    if gone || unclaimed {
        record.operation.status = OperationStatus::Unknown;
        record.operation.error = Some("Execution host is no longer available; inspect the Agent before issuing new work".into());
        return true;
    }
    false
}

impl Record {
    pub(crate) fn observe(mut self, context: &RequestContext) -> Operation {
        reconcile_view(context, &mut self);
        self.operation
    }
}

fn reconcile(db: &AgentxDb, context: &RequestContext, record: &mut Record) -> ServiceResult<()> {
    if reconcile_view(context, record) {
        save(db, record)?;
    }
    Ok(())
}
pub(crate) fn validate_reference(reference: &AgentRef) -> ServiceResult<()> {
    if reference.name.is_empty() || reference.name.len() > 128 || !reference.name.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)) {
        return Err(invalid("invalid Agent name"));
    }
    match (&reference.incarnation, &reference.stopped_event) {
        (Some(bits), None) if bits.parse::<u64>().is_ok() => Ok(()),
        (None, Some(event)) if event.parse::<i64>().is_ok_and(|id| id > 0) => Ok(()),
        _ => Err(invalid("supply exactly one incarnation or stopped_event")),
    }
}
pub(super) fn stopped_snapshot(db: &AgentxDb, reference: &AgentRef) -> ServiceResult<Value> {
    let event = reference.stopped_event.as_deref().ok_or(ServiceError::StaleFence)?.parse::<i64>().map_err(|_| invalid("invalid stop reference"))?;
    let value: Option<String> =
        db.conn().query_row("SELECT data FROM events WHERE id=?1 AND instance=?2 AND type='life' AND json_extract(data,'$.action')='stopped'", rusqlite::params![event, reference.name], |row| row.get(0)).optional().map_err(internal)?;
    let data: Value = serde_json::from_str(&value.ok_or(ServiceError::StaleFence)?).map_err(internal)?;
    data.get("snapshot").filter(|value| value.is_object()).cloned().ok_or(ServiceError::StaleFence)
}
fn tool_resume(tool: &str, session: Option<&str>) -> bool {
    session.is_some_and(|s| !s.is_empty()) && crate::runtime::launcher::LaunchTool::from_str(tool).is_ok_and(|t| t.spec().resume.is_some())
}
fn can_fork(tool: &str, session: Option<&str>) -> bool {
    session.is_some_and(|value| !value.is_empty()) && crate::runtime::launcher::LaunchTool::from_str(tool).is_ok_and(|tool| tool.spec().resume.and_then(|resume| resume.fork).is_some())
}
fn live_view(db: &AgentxDb, row: &InstanceRow, snapshot: Option<&labels::LabelSnapshot>, pending_messages: Option<u64>) -> AgentView {
    let state = crate::agents::lifecycle::get_instance_status_stored(row, db);
    let running = row.pid.is_some_and(|pid| pid > 0) && !row.status_context.starts_with("exit:");
    AgentView {
        labels: snapshot.map(|value| value.labels.clone()),
        labels_revision: snapshot.map(|value| value.revision.clone()),
        pending_messages,
        title: crate::agents::launch_title::from_context(row.launch_context.as_deref()),
        file_bindings: Vec::new(),
        agent: AgentRef { name: row.name.clone(), incarnation: Some(row.created_at.to_bits().to_string()), stopped_event: None },
        tool: row.tool.clone(),
        status: state.status,
        stored_status: row.status.clone(),
        status_context: state.context,
        request_error: row.model_request.as_ref().filter(|request| row.session_id.as_deref() == Some(request.session_id.as_str())).and_then(|request| request.error.clone()),
        detail: state.description,
        directory: row.directory.clone(),
        session_id: row.session_id.clone(),
        collaboration: super::offices::collaboration(row.launch_context.as_deref()),
        can_send: running || matches!(row.status.as_str(), "active" | "listening" | "blocked"),
        can_stop: running,
        can_stop_participation: true,
        can_fork: can_fork(&row.tool, row.session_id.as_deref()),
        can_resume: !running && tool_resume(&row.tool, row.session_id.as_deref()),
        has_transcript: !row.transcript_path.is_empty(),
    }
}
pub(crate) fn agent(db: &AgentxDb, reference: &AgentRef) -> ServiceResult<AgentView> {
    validate_reference(reference)?;
    if let Some(bits) = &reference.incarnation {
        let row = db.get_instance_full(&reference.name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        if row.created_at.to_bits().to_string() != *bits || row.origin_device_id.is_some() {
            return Err(ServiceError::StaleFence);
        }
        // Reference/capability checks do not need a message scan. Detail reads
        // explicitly hydrate pending counts in projected_agent below.
        Ok(live_view(db, &row, None, None))
    } else {
        let snapshot = stopped_snapshot(db, reference)?;
        let text = |key: &str| snapshot.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
        let tool = text("tool");
        let session = text("session_id");
        Ok(AgentView {
            labels: snapshot.get("labels").cloned().map(serde_json::from_value).transpose().map_err(internal)?,
            labels_revision: snapshot.get("labels_revision").and_then(Value::as_str).map(str::to_owned),
            pending_messages: None,
            title: crate::agents::launch_title::from_snapshot(&snapshot),
            file_bindings: Vec::new(),
            agent: reference.clone(),
            status: "stopped".into(),
            stored_status: text("status"),
            status_context: text("status_context"),
            request_error: snapshot.get("model_request").cloned().and_then(|value| serde_json::from_value::<crate::agents::model_request::ModelRequestState>(value).ok()).and_then(|request| request.error),
            detail: "Stopped".into(),
            directory: text("directory"),
            can_resume: db.get_instance_full(&reference.name).map_err(internal)?.is_none() && tool_resume(&tool, Some(&session)),
            can_fork: can_fork(&tool, Some(&session)),
            tool,
            session_id: (!session.is_empty()).then_some(session),
            collaboration: super::offices::collaboration_value(&snapshot),
            can_send: false,
            can_stop: false,
            can_stop_participation: false,
            has_transcript: !text("transcript_path").is_empty(),
        })
    }
}
fn participant(reference: &AgentRef) -> ServiceResult<ParticipantRef> {
    validate_reference(reference)?;
    Ok(ParticipantRef { canonical_name: reference.name.clone(), incarnation_bits: reference.incarnation.as_deref().ok_or_else(|| invalid("labels require a current exact Agent"))?.parse().map_err(|_| invalid("invalid incarnation"))? })
}
fn label_snapshot(snapshot: labels::LabelSnapshot) -> LabelsSnapshot {
    LabelsSnapshot {
        generation: snapshot.generation,
        agent: AgentRef { name: snapshot.participant.canonical_name, incarnation: Some(snapshot.participant.incarnation_bits.to_string()), stopped_event: None },
        labels: snapshot.labels,
        revision: snapshot.revision,
    }
}
fn hydrate_views<'a>(db: &AgentxDb, context: &RequestContext, views: impl Iterator<Item = &'a mut AgentView>) -> ServiceResult<()> {
    let mut views: Vec<_> = views.collect();
    let participants = views.iter().filter(|view| view.agent.incarnation.is_some()).map(|view| participant(&view.agent)).collect::<ServiceResult<Vec<_>>>()?;
    let snapshots = labels::hydrate(db, context, &participants)?;
    for view in &mut views {
        if view.agent.incarnation.is_some()
            && let Some(snapshot) = snapshots.get(&view.agent.name)
        {
            view.labels = Some(snapshot.labels.clone());
            view.labels_revision = Some(snapshot.revision.clone());
        }
    }
    Ok(())
}
pub(super) fn projected_agent(db: &AgentxDb, context: &RequestContext, reference: &AgentRef) -> ServiceResult<AgentView> {
    let mut view = agent(db, reference)?;
    if reference.incarnation.is_some() {
        let row = db.get_instance_full(&reference.name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        let unread = crate::service::queries_store::listing::try_get_unread_counts_batch(db, std::slice::from_ref(&row))?;
        view.pending_messages = unread.get(&row.name).and_then(|count| u64::try_from(*count).ok());
    }
    hydrate_views(db, context, std::iter::once(&mut view))?;
    Ok(view)
}
/// Preserve the catalog's existing first ten pages and ordering, projecting only
/// collaboration context. No unread counts, labels, status or transcript facts are needed.
fn adopted_workspaces(db: &AgentxDb) -> ServiceResult<AdoptedWorkspaces> {
    const LIMIT: usize = 1000;
    let mut contexts = Vec::new();
    let mut stmt = db.conn().prepare("SELECT launch_context FROM instances WHERE COALESCE(origin_device_id,'')='' ORDER BY name LIMIT ?1").map_err(internal)?;
    for context in stmt.query_map([(LIMIT + 1) as i64], |row| row.get::<_, Option<String>>(0)).map_err(internal)? {
        contexts.push(context.map_err(internal)?);
    }
    if contexts.len() <= LIMIT {
        let mut stmt = db.conn().prepare("SELECT json_extract(data,'$.snapshot.launch_context') FROM events WHERE type='life' AND json_extract(data,'$.action')='stopped' AND json_type(data,'$.snapshot')='object' AND COALESCE(json_extract(data,'$.snapshot.origin_device_id'),'')='' ORDER BY id DESC LIMIT ?1").map_err(internal)?;
        for context in stmt.query_map([(LIMIT + 1 - contexts.len()) as i64], |row| row.get::<_, Option<String>>(0)).map_err(internal)? {
            contexts.push(context.map_err(internal)?);
        }
    }
    let agent_limit_reached = contexts.len() > LIMIT;
    contexts.truncate(LIMIT);
    let agent_pages = (contexts.len().max(1).div_ceil(100)) as u32;
    let mut seen = std::collections::HashSet::new();
    let paths = contexts
        .iter()
        .filter_map(|context| {
            let collaboration = super::offices::collaboration(context.as_deref());
            (collaboration.status == "adopted").then_some(collaboration.workspace).flatten()
        })
        // Preserve first occurrence: host discovery caps admitted workspaces,
        // so sorting before admission could displace an existing current workspace.
        .filter(|path| seen.insert(path.clone()))
        .collect();
    Ok(AdoptedWorkspaces { paths, agent_pages, agent_limit_reached })
}

fn list(db: &AgentxDb, context: &RequestContext, after: Option<String>, limit: u32) -> ServiceResult<AgentPage> {
    if limit == 0 || limit > MAX_PAGE || after.as_ref().is_some_and(|s| s.len() > 300) {
        return Err(invalid("invalid page bounds"));
    }
    list_page(db, context, after.as_deref().unwrap_or(""), limit, None, false)
}

fn workspace_members(db: &AgentxDb, context: &RequestContext, workspace: &str, current_only: bool, after: Option<&str>, limit: u32) -> ServiceResult<AgentPage> {
    use sha2::{Digest, Sha256};
    if workspace.len() > 4096 || !crate::agents::workspaces::is_workspace_key(workspace) {
        return Err(invalid("workspace must be global or an absolute saved workspace path"));
    }
    if limit == 0 || limit > MAX_PAGE || after.is_some_and(|s| s.len() > 300) {
        return Err(invalid("invalid workspace member page bounds"));
    }
    let binding = serde_json::to_vec(&(&context.database_generation, workspace, current_only)).map_err(internal)?;
    let fingerprint: String = Sha256::digest(binding).iter().map(|byte| format!("{byte:02x}")).collect();
    let prefix = format!("workspace-members:{fingerprint}:");
    let cursor = match after {
        None => "",
        Some(cursor) => cursor.strip_prefix(&prefix).filter(|cursor| cursor.starts_with("0:") || (!current_only && cursor.starts_with("1:"))).ok_or_else(|| invalid("workspace member cursor belongs to another query or generation"))?,
    };
    let mut page = list_page(db, context, cursor, limit, Some(workspace), current_only)?;
    page.next = page.next.map(|cursor| format!("{prefix}{cursor}"));
    Ok(page)
}

fn list_page(db: &AgentxDb, context: &RequestContext, cursor: &str, limit: u32, workspace: Option<&str>, current_only: bool) -> ServiceResult<AgentPage> {
    let (live_after, stop_before) = if cursor.is_empty() {
        (Some(""), i64::MAX)
    } else if let Some(name) = cursor.strip_prefix("0:").filter(|name| !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))) {
        (Some(name), i64::MAX)
    } else if let Some(value) = cursor.strip_prefix("1:").filter(|value| value.len() == 20) {
        let inverted = value.parse::<i64>().ok().filter(|value| *value >= 0 && *value < i64::MAX).ok_or_else(|| invalid("invalid Agent cursor"))?;
        (None, i64::MAX - inverted)
    } else {
        return Err(invalid("invalid Agent cursor"));
    };
    let mut entries = Vec::new();
    if let Some(live_after) = live_after {
        // Native rows represent local origin as NULL or an empty string;
        // match InstanceRow normalization before applying pagination.
        let names = if let Some(workspace) = workspace {
            // The existing adopted workspace/name index selects candidates;
            // the checked snapshot predicate excludes unsupported adoption.
            let indexed_key = (workspace != crate::agents::workspaces::GLOBAL_WORKSPACE_KEY).then_some(workspace);
            let mut stmt = db.conn().prepare_cached("SELECT name FROM instances WHERE COALESCE(origin_device_id,'')='' AND name > ?1 AND (CASE WHEN json_valid(launch_context) THEN json_extract(launch_context,'$.collaboration.workspace') END) IS ?3 AND agentx_adopted_workspace(launch_context)=?4 ORDER BY name LIMIT ?2").map_err(internal)?;
            let rows = stmt.query_map(rusqlite::params![live_after, limit + 1, indexed_key, workspace], |r| r.get::<_, String>(0)).map_err(internal)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(internal)?
        } else {
            let mut stmt = db.conn().prepare_cached("SELECT name FROM instances WHERE COALESCE(origin_device_id,'')='' AND name > ?1 ORDER BY name LIMIT ?2").map_err(internal)?;
            let rows = stmt.query_map(rusqlite::params![live_after, limit + 1], |r| r.get::<_, String>(0)).map_err(internal)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(internal)?
        };
        let mut rows = Vec::new();
        for name in names {
            if let Some(row) = db.get_instance_full(&name).map_err(internal)? {
                rows.push(row);
            }
        }
        let unread = crate::service::queries_store::listing::try_get_unread_counts_batch(db, &rows)?;
        for row in rows {
            let pending = unread.get(&row.name).and_then(|count| u64::try_from(*count).ok());
            entries.push((format!("0:{}", row.name), live_view(db, &row, None, pending)));
        }
    }
    if !current_only && entries.len() <= limit as usize {
        // Distinct exact stopped executions remain visible even if their name is reused.
        let remaining = i64::from(limit) + 1 - entries.len() as i64;
        let stopped_row = |row: &rusqlite::Row<'_>| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?));
        let stopped = if let Some(workspace) = workspace {
            // An old stopped snapshot may store launch context as a JSON object
            // instead of a serialized string, just as the detail view accepts.
            let mut stmt = db
                .conn()
                .prepare_cached(
                    "SELECT id,instance FROM events WHERE id < ?1 AND type='life' AND CASE WHEN json_valid(data) THEN
                    json_extract(data,'$.action')='stopped' AND json_type(data,'$.snapshot')='object' AND COALESCE(json_extract(data,'$.snapshot.origin_device_id'),'')=''
                    AND agentx_adopted_workspace(CASE WHEN json_type(data,'$.snapshot.launch_context') IN ('text','object') THEN json_extract(data,'$.snapshot.launch_context') END)=?3
                 END ORDER BY id DESC LIMIT ?2",
                )
                .map_err(internal)?;
            let rows = stmt.query_map(rusqlite::params![stop_before, remaining, workspace], stopped_row).map_err(internal)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(internal)?
        } else {
            let mut stmt = db.conn().prepare_cached("SELECT id,instance FROM events WHERE id < ?1 AND type='life' AND json_extract(data,'$.action')='stopped' AND json_type(data,'$.snapshot')='object' AND COALESCE(json_extract(data,'$.snapshot.origin_device_id'),'')='' ORDER BY id DESC LIMIT ?2").map_err(internal)?;
            let rows = stmt.query_map(rusqlite::params![stop_before, remaining], stopped_row).map_err(internal)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(internal)?
        };
        for (id, name) in stopped {
            let entry_key = format!("1:{:020}", i64::MAX - id);
            let reference = AgentRef { name, incarnation: None, stopped_event: Some(id.to_string()) };
            entries.push((entry_key, agent(db, &reference)?));
        }
    }
    let current_complete = !entries.iter().skip(limit as usize).any(|(key, _)| key.starts_with("0:"));
    let more = entries.len() > limit as usize;
    entries.truncate(limit as usize);
    hydrate_views(db, context, entries.iter_mut().map(|(_, view)| view))?;
    Ok(AgentPage { current_complete, generation: context.database_generation.clone(), next: more.then(|| entries.last().unwrap().0.clone()), agents: entries.into_iter().map(|(_, view)| view).collect() })
}
fn conversation_bounds(db: &AgentxDb, reference: &AgentRef) -> ServiceResult<(ParticipantRef, String, Option<i64>)> {
    // Native messages have daemon timestamps. Bound both sides of a stopped
    // execution so a reused name cannot merge unrelated conversation history.
    let created =
        if let Some(bits) = &reference.incarnation { f64::from_bits(bits.parse().map_err(|_| invalid("invalid incarnation"))?) } else { stopped_snapshot(db, reference)?.get("created_at").and_then(Value::as_f64).ok_or(ServiceError::StaleFence)? };
    if !created.is_finite() || created < 0.0 {
        return Err(ServiceError::StaleFence);
    }
    let start = chrono::DateTime::from_timestamp(created as i64, (created.fract() * 1_000_000_000.0) as u32).ok_or(ServiceError::StaleFence)?.format("%Y-%m-%dT%H:%M:%S%.6f+00:00").to_string();
    let through = reference.stopped_event.as_deref().map(|id| id.parse::<i64>().map_err(|_| invalid("invalid stopped event"))).transpose()?;
    Ok((ParticipantRef { canonical_name: reference.name.clone(), incarnation_bits: created.to_bits() }, start, through))
}

fn messages(db: &AgentxDb, context: &RequestContext, reference: AgentRef, after: Option<String>, limit: u32) -> ServiceResult<MessagePage> {
    agent(db, &reference)?;
    if limit == 0 || limit > MAX_PAGE {
        return Err(invalid("invalid page size"));
    }
    let after = after.as_deref().unwrap_or("0").parse::<i64>().map_err(|_| invalid("invalid event cursor"))?;
    if after < 0 {
        return Err(invalid("invalid event cursor"));
    }
    let (participant, start, through) = conversation_bounds(db, &reference)?;
    let page = crate::events::store::conversation_page(db, context, &participant, &start, after, through, limit as usize)?;
    let messages = project_messages(db, context, page.events)?;
    Ok(MessagePage { generation: context.database_generation.clone(), messages, next_after: page.next_after.to_string(), has_more: page.has_more })
}

fn message_history(db: &AgentxDb, context: &RequestContext, reference: AgentRef, before: Option<String>, limit: u32) -> ServiceResult<MessageHistoryPage> {
    // The production read lane already pins a WAL snapshot, including the generation check.
    let _snapshot = if db.conn().is_autocommit() { Some(db.conn().unchecked_transaction().map_err(internal)?) } else { None };
    agent(db, &reference)?;
    if limit == 0 || limit > MAX_PAGE {
        return Err(invalid("invalid page size"));
    }
    let before = before.map(|value| value.parse::<i64>().ok().filter(|id| *id > 0).ok_or_else(|| invalid("invalid event cursor"))).transpose()?;
    let (participant, start, stop) = conversation_bounds(db, &reference)?;
    let live_after = if before.is_none() { Some(db.conn().query_row("SELECT COALESCE(MAX(id),0) FROM events", [], |row| row.get::<_, i64>(0)).map_err(internal)?) } else { None };
    let through = before.map(|id| id - 1).or(live_after).unwrap_or_default().min(stop.unwrap_or(i64::MAX));
    let (events, has_more) = crate::events::store::conversation_history(db, &participant, &start, through, limit as usize)?;
    let messages = project_messages(db, context, events)?;
    let next_before = if has_more { messages.first().map(|message| message.id.clone()) } else { None };
    Ok(MessageHistoryPage { generation: context.database_generation.clone(), messages, next_before, live_after: live_after.map(|id| id.to_string()) })
}

fn project_messages(db: &AgentxDb, context: &RequestContext, events: Vec<Value>) -> ServiceResult<Vec<MessageView>> {
    events
        .into_iter()
        .map(|event| {
            let data = event.get("data").cloned().unwrap_or_default();
            let string = |key: &str| data.get(key).and_then(Value::as_str).map(str::to_owned);
            Ok(MessageView {
                attachments: message_attachments(db, &context.database_generation, &event.get("id").map(|v| v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string())).unwrap_or_default())?,
                id: event.get("id").map(|v| v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string())).unwrap_or_default(),
                at: event.get("ts").and_then(Value::as_str).unwrap_or_default().into(),
                from: string("from").unwrap_or_default(),
                text: string("text").unwrap_or_default(),
                recipients: data.get("delivered_to").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect(),
                intent: string("intent"),
                reply_to: data.get("reply_to").map(|v| v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string())),
                thread: string("thread"),
            })
        })
        .collect()
}

fn attachment_key(generation: &str, id: &str) -> String {
    format!("management:attachment:{generation}:{id}")
}
fn attachment(db: &AgentxDb, generation: &str, id: &str) -> ServiceResult<StoredAttachment> {
    uuid::Uuid::parse_str(id).map_err(|_| invalid("invalid attachment ID"))?;
    let value = db.kv_get(&attachment_key(generation, id)).map_err(internal)?.ok_or_else(|| invalid("attachment not registered in this generation"))?;
    serde_json::from_str(&value).map_err(internal)
}
fn message_attachments(db: &AgentxDb, generation: &str, id: &str) -> ServiceResult<Vec<Attachment>> {
    db.kv_get(&format!("management:message-attachments:{generation}:{id}")).map_err(internal)?.map(|value| serde_json::from_str(&value).map_err(internal)).transpose().map(|items| items.unwrap_or_default())
}
fn resolve_attachments(db: &AgentxDb, generation: &str, message: &MessageInput) -> ServiceResult<Vec<StoredAttachment>> {
    if message.attachments.len() > super::uploads::MAX_FILES {
        return Err(invalid("at most 8 attachments per message"));
    }
    let mut seen = std::collections::HashSet::new();
    message
        .attachments
        .iter()
        .map(|id| {
            if !seen.insert(id) {
                return Err(invalid("duplicate attachment ID"));
            }
            attachment(db, generation, id)
        })
        .collect()
}
fn validate_message(message: &MessageInput) -> ServiceResult<()> {
    if !message.text.trim().is_empty() || message.attachments.is_empty() {
        crate::messaging::routing::validate_message(&message.text).map_err(ServiceError::InvalidInput)?;
    }
    if message.text.len() > MAX_TEXT {
        return Err(invalid("message exceeds limit"));
    }
    if [&message.reply_to, &message.thread].into_iter().flatten().any(|value| value.len() > 512 || value.contains('\0')) {
        return Err(invalid("invalid message metadata"));
    }
    if message.intent.as_deref().is_some_and(|value| !["request", "inform", "ack"].contains(&value)) {
        return Err(invalid("invalid intent"));
    }
    Ok(())
}

fn validate_command(db: &AgentxDb, generation: &str, command: &Command) -> ServiceResult<()> {
    match command {
        Command::Launch(input) => {
            labels::validate_set(&input.labels)?;
            crate::agents::launch_title::normalize(input.title.as_deref()).map_err(|error| invalid(&error))?;
            if input.role.is_none() && input.tool.is_none() && input.profile.is_none() {
                return Err(invalid("choose a role, tool or profile"));
            }
            if input.role.is_some() && (input.tool.is_some() || input.profile.is_some()) {
                return Err(invalid("role conflicts with explicit tool or profile"));
            }
            if input.workspace.as_ref().is_some_and(|value| value != "global" && !std::path::Path::new(value).is_absolute()) {
                return Err(invalid("workspace must be absolute or global"));
            }
            if input.role.as_deref().is_some_and(str::is_empty) || input.workspace.as_deref().is_some_and(str::is_empty) {
                return Err(invalid("role and workspace cannot be empty"));
            }
            for value in [&input.role, &input.workspace, &input.tool, &input.profile, &input.directory, &input.prompt].into_iter().flatten() {
                if value.len() > MAX_TEXT || value.contains('\0') {
                    return Err(invalid("invalid launch text"));
                }
            }
        }
        Command::Send { agent: reference, message } => {
            if !agent(db, reference)?.can_send {
                return Err(ServiceError::Conflict("Agent is not available for messages".into()));
            }
            validate_message(message)?;
            resolve_attachments(db, generation, message)?;
        }
        Command::DirectorySend { directory, message } => {
            if !message.attachments.is_empty() {
                return Err(invalid("directory send supports text only"));
            }
            if directory.is_empty() || directory.len() > MAX_TEXT || directory.contains('\0') || !std::path::Path::new(directory).is_absolute() {
                return Err(invalid("directory must be an absolute path"));
            }
            validate_message(message)?;
        }
        Command::StopParticipation(reference) if !agent(db, reference)?.can_stop_participation => return Err(ServiceError::Conflict("Agent cannot stop participation".into())),
        Command::Fork(reference) if !agent(db, reference)?.can_fork => return Err(ServiceError::Conflict("Agent cannot fork its session".into())),
        Command::Stop(reference) if !agent(db, reference)?.can_stop => return Err(ServiceError::Conflict("Agent is not running".into())),
        Command::Resume(reference) if !agent(db, reference)?.can_resume => return Err(ServiceError::Conflict("Agent cannot resume".into())),
        Command::Transcript(reference) => {
            agent(db, reference)?;
        }
        _ => {}
    }
    Ok(())
}
fn worker_matches(record: &Record, peer: &CallerProcess) -> bool {
    record.worker.as_ref().is_some_and(|(pid, incarnation)| *pid == peer.pid && peer.incarnation.as_ref() == Some(incarnation))
}
fn result_matches(command: &Command, result: &CommandResult) -> bool {
    match (command, result) {
        (Command::Launch(_) | Command::Resume(_) | Command::Fork(_), CommandResult::Launched(result)) => result
            .agent_refs
            .as_ref()
            .is_none_or(|refs| !refs.is_empty() && refs.len() == result.agents.len() && refs.iter().zip(&result.agents).all(|(reference, name)| &reference.name == name && reference.incarnation.is_some() && validate_reference(reference).is_ok())),
        (Command::Send { agent, .. }, CommandResult::Sent(result)) => result.recipients == [agent.name.clone()],
        (Command::DirectorySend { .. }, CommandResult::DirectorySent(result)) => {
            validate_reference(&result.recipient).is_ok() && result.recipient.incarnation.is_some() && result.sent.recipients == [result.recipient.name.clone()] && result.entry.endpoint.participant.as_ref() == Some(&result.recipient)
        }
        (Command::Stop(agent) | Command::StopParticipation(agent), CommandResult::Stopped(result)) => agent.name == result.name,
        (Command::Transcript(agent), CommandResult::Transcript(result)) => agent == &result.agent,
        _ => false,
    }
}
pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    match request {
        Request::RegisterAttachment(stored) => {
            uuid::Uuid::parse_str(&stored.attachment.id).map_err(|_| invalid("invalid attachment ID"))?;
            let key = attachment_key(&context.database_generation, &stored.attachment.id);
            if db.kv_get(&key).map_err(internal)?.is_some() {
                return Err(invalid("attachment already registered"));
            }
            db.kv_set(&key, Some(&serde_json::to_string(&stored).map_err(internal)?)).map_err(internal)?;
            return Ok(Response::Attachment(stored));
        }
        Request::Attachment(id) => return attachment(db, &context.database_generation, &id).map(Response::Attachment),
        Request::Send { agent: reference, message } => {
            return transaction(db, || {
                use crate::messaging::inbox::ProductSend;
                let view = agent(db, &reference)?;
                if !view.can_send || reference.incarnation.is_none() {
                    return Err(ServiceError::StaleFence);
                }
                let row = db.get_instance_full(&reference.name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
                let product = ProductSend::execution(&row)
                    .map_err(internal)?
                    .map(|execution| {
                        let request = &context.caller.request_id.0;
                        let record = load(db, context, request)?;
                        if uuid::Uuid::parse_str(request).is_err()
                            || !worker_matches(&record, &context.process)
                            || record.operation.status != OperationStatus::Running
                            || record.command != (Command::Send { agent: reference.clone(), message: message.clone() })
                        {
                            return Err(ServiceError::Forbidden("dedicated execution requires its original exact management send".into()));
                        }
                        Ok(ProductSend { participant: ParticipantRef { canonical_name: row.name, incarnation_bits: row.created_at.to_bits() }, execution, request_id: request.clone() })
                    })
                    .transpose()?;
                validate_message(&message)?;
                let stored = resolve_attachments(db, &context.database_generation, &message)?;
                let mut text = message.text.clone();
                if !stored.is_empty() {
                    text.push_str("\n\nAttachments on the AgentX host (use available image/file reading tools; paths alone do not inject Provider media; remote filesystems may differ):");
                    for item in &stored {
                        text.push_str(&format!("\n- {} [{}; {} bytes]: {}", item.attachment.name, item.attachment.media_type, item.attachment.size, item.path));
                    }
                }
                let envelope =
                    crate::messaging::routing::MessageEnvelope { intent: message.intent.as_deref().map(str::parse).transpose().map_err(ServiceError::InvalidInput)?, reply_to: message.reply_to, thread: message.thread, ..Default::default() };
                let result = crate::messaging::service::handle(
                    db,
                    context,
                    crate::messaging::service::Request::Submit(crate::messaging::service::Submission { recipients: vec![reference.name], text, envelope, external_sender: Some("bigboss".into()), broadcast_confirmed: false, inline_bundle: None }),
                )?;
                let crate::messaging::service::Response::Accepted(accepted) = result else {
                    return Err(internal("unexpected message response"));
                };
                if let Some(source) = product {
                    if accepted.recipients != [source.participant.canonical_name.clone()] {
                        return Err(ServiceError::StaleFence);
                    }
                    let key = ProductSend::key(&context.database_generation, accepted.event.0);
                    let original = serde_json::to_string(&source).map_err(internal)?;
                    if db.kv_get(&key).map_err(internal)?.is_some_and(|stored| stored != original) {
                        return Err(ServiceError::StaleFence);
                    }
                    db.kv_set(&key, Some(&original)).map_err(internal)?;
                }
                let refs: Vec<Attachment> = stored.into_iter().map(|item| item.attachment).collect();
                db.kv_set(&format!("management:message-attachments:{}:{}", context.database_generation, accepted.event.0), Some(&serde_json::to_string(&refs).map_err(internal)?)).map_err(internal)?;
                Ok(Response::Sent(SendOutcome { message_id: accepted.event.0.to_string(), recipients: accepted.recipients }))
            });
        }
        Request::TranscriptSource(reference) => {
            agent(db, &reference)?;
            let selected = if let Some(bits) = &reference.incarnation {
                crate::transcript::store::select(db, &reference.name, Some(&crate::agents::attachments::ParticipantRef { canonical_name: reference.name.clone(), incarnation_bits: bits.parse().map_err(|_| invalid("invalid incarnation"))? }))?
            } else {
                let snapshot = stopped_snapshot(db, &reference)?;
                let text = |key: &str| snapshot.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
                let path = text("transcript_path");
                let session = text("session_id");
                crate::transcript::service::Selection {
                    canonical_name: reference.name.clone(),
                    missing: None,
                    codex_lookup: None,
                    source: (!path.is_empty()).then(|| crate::transcript::service::Source { name: reference.name, path, tool: text("tool"), session_id: (!session.is_empty()).then_some(session) }),
                }
            };
            return Ok(Response::TranscriptSource(selected));
        }
        Request::HumanMailbox { before, pending, limit } => return super::human::read(db, context, before, pending, limit).map(Response::HumanMailbox),
        Request::HandleHumanMessages { ids } => return super::human::handle(db, context, ids).map(Response::HumanMailbox),
        Request::OfficeActivity { after, limit } => return super::activity::read(db, context, after, limit).map(Response::OfficeActivity),
        Request::AdoptedWorkspaces => return adopted_workspaces(db).map(Response::AdoptedWorkspaces),
        Request::List { after, limit } => return list(db, context, after, limit).map(Response::Agents),
        Request::ListWorkspaceMembers { workspace, current_only, after, limit } => return workspace_members(db, context, &workspace, current_only, after.as_deref(), limit).map(Response::Agents),
        Request::SelectLabels { after, limit, selectors } => {
            let predicates = selectors.iter().map(|text| labels::parse_selector(text)).collect::<ServiceResult<Vec<_>>>()?;
            let page = labels::select_current(db, context, labels::LabelQuery { selectors: predicates, after, limit, ..Default::default() })?;
            let unread = crate::service::queries_store::listing::try_get_unread_counts_batch(db, &page.records)?;
            let agents = page.records.iter().map(|row| live_view(db, row, page.snapshots.get(&row.name), unread.get(&row.name).and_then(|count| u64::try_from(*count).ok()))).collect();
            return Ok(Response::Agents(AgentPage { current_complete: page.next.is_none(), generation: context.database_generation.clone(), agents, next: page.next }));
        }
        Request::Labels(reference) => return labels::get(db, context, &participant(&reference)?).map(|snapshot| Response::Labels(label_snapshot(snapshot))),
        Request::Agent(reference) => return projected_agent(db, context, &reference).map(Response::Agent),
        Request::LaunchRelations { agent, after, limit } => return super::relations::read(db, context, agent, after, limit).map(Response::LaunchRelations),
        Request::Messages { agent, after, limit } => return messages(db, context, agent, after, limit).map(Response::Messages),
        Request::MessageHistory { agent, before, limit } => return message_history(db, context, agent, before, limit).map(Response::MessageHistory),
        _ => {}
    }
    transaction(db, || match request {
        Request::Submit { request_id, command, admit } => {
            uuid::Uuid::parse_str(&request_id).map_err(|_| invalid("request_id must be a UUID"))?;
            match load(db, context, &request_id) {
                Ok(mut record) => {
                    if record.command != command {
                        return Err(ServiceError::Conflict("request_id already belongs to another command".into()));
                    }
                    reconcile(db, context, &mut record)?;
                    return Ok(Response::Grant(WorkGrant { operation: record.operation, ticket: None }));
                }
                Err(ServiceError::NotFound(_)) => {}
                Err(error) => return Err(error),
            }
            if let Command::PatchLabels(patch) = &command {
                let native = labels::PatchLabels { participant: participant(&patch.agent)?, set: patch.set.clone(), remove: patch.remove.clone(), if_revision: patch.if_revision.clone() };
                let mut label_context = context.clone();
                label_context.caller.request_id = crate::service::RequestId(format!("management-labels:{request_id}"));
                let labels::Response::Patched(result) = labels::handle(db, &label_context, labels::Request::Patch(native))? else { return Err(internal("unexpected labels patch result")) };
                let record = Record {
                    operation: Operation {
                        request_id,
                        generation: context.database_generation.clone(),
                        status: OperationStatus::Succeeded,
                        result: Some(CommandResult::LabelsPatched(LabelsOutcome { snapshot: label_snapshot(result.snapshot), changed: result.changed })),
                        error: None,
                    },
                    command,
                    ticket: String::new(),
                    daemon: context.daemon_instance.clone(),
                    worker: None,
                    accepted_at: crate::shared::time::now_epoch_i64(),
                };
                save(db, &record)?;
                return Ok(Response::Grant(WorkGrant { operation: record.operation, ticket: None }));
            }
            if !admit {
                return Err(ServiceError::Unavailable("management workers busy".into()));
            }
            validate_command(db, &context.database_generation, &command)?;
            let record = Record {
                operation: Operation { request_id, generation: context.database_generation.clone(), status: OperationStatus::Accepted, result: None, error: None },
                command,
                ticket: uuid::Uuid::new_v4().to_string(),
                daemon: context.daemon_instance.clone(),
                worker: None,
                accepted_at: crate::shared::time::now_epoch_i64(),
            };
            save(db, &record)?;
            Ok(Response::Grant(WorkGrant { operation: record.operation, ticket: Some(record.ticket) }))
        }
        Request::Operation { request_id } => Ok(Response::Operation(operation_record(db, context, &request_id)?.observe(context))),
        Request::Claim { request_id, ticket } => {
            let mut record = load(db, context, &request_id)?;
            reconcile(db, context, &mut record)?;
            if record.ticket != ticket || record.operation.status != OperationStatus::Accepted {
                return Err(ServiceError::Conflict("operation already claimed or ticket invalid".into()));
            }
            let incarnation = context.process.incarnation.clone().filter(|v| !v.is_empty()).ok_or_else(|| ServiceError::Forbidden("worker incarnation unavailable".into()))?;
            validate_command(db, &context.database_generation, &record.command)?;
            record.worker = Some((context.process.pid, incarnation));
            record.operation.status = OperationStatus::Running;
            save(db, &record)?;
            Ok(Response::Claimed(record.command))
        }
        Request::Finish { request_id, ticket, result } => {
            let mut record = load(db, context, &request_id)?;
            if record.ticket != ticket || !worker_matches(&record, &context.process) {
                return Err(ServiceError::Forbidden("operation belongs to another worker".into()));
            }
            if result.as_ref().is_ok_and(|result| !result_matches(&record.command, result) || matches!(result, CommandResult::DirectorySent(sent) if sent.entry.generation != context.database_generation)) {
                return Err(invalid("result does not match accepted command"));
            }
            if active(&record) || record.operation.status == OperationStatus::Unknown {
                match result {
                    Ok(result) if serde_json::to_vec(&result).map_err(internal)?.len() <= MAX_RESULT_BYTES => {
                        record.operation.status = OperationStatus::Succeeded;
                        record.operation.result = Some(result);
                        record.operation.error = None;
                    }
                    Ok(_) => {
                        record.operation.status = OperationStatus::Failed;
                        record.operation.error = Some("result exceeds management limit".into());
                    }
                    Err(error) => {
                        record.operation.status = if error.starts_with("outcome_unknown:") { OperationStatus::Unknown } else { OperationStatus::Failed };
                        record.operation.error = Some(error.chars().take(4096).collect());
                    }
                }
                save(db, &record)?;
            }
            Ok(Response::Operation(record.operation))
        }
        Request::Abandon { request_id, ticket, reason } => {
            let mut record = load(db, context, &request_id)?;
            if record.ticket != ticket {
                return Err(ServiceError::Forbidden("invalid operation ticket".into()));
            }
            if active(&record) {
                record.operation.status = OperationStatus::Unknown;
                record.operation.error = Some(reason.chars().take(4096).collect());
                save(db, &record)?;
            }
            Ok(Response::Operation(record.operation))
        }
        _ => unreachable!(),
    })
}
