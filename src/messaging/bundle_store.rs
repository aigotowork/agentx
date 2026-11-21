//! Daemon-only context bundle persistence; see bundle_service for host boundaries.
use super::bundle_service::*;
use crate::db::AgentxDb;
use crate::service::{RequestContext, ServiceResult};

pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    match request {
        Request::Lookup { id } => lookup(db, &id).map(Response::Lookup),
        Request::List { before, through, limit } => list(db, before, through, limit).map(Response::Page),
        Request::Context { agent, category, before, through, limit } => context_events(db, &agent, category, before, through, limit).map(Response::Page),
        Request::ReferencedEvents { refs, after, through, limit } => referenced(db, &refs, after, through, limit).map(Response::Page),
        Request::Create { bundle, actor } => create(db, context, bundle, actor).map(Response::Created),
    }
}

use crate::agents::attachments::ParticipantRef;
use crate::events::service::{MAX_PAGE_BYTES, MAX_PAGE_EVENTS};
use crate::service::{InvocationContext, ServiceError};
use rusqlite::{OptionalExtension, types::Value as SqlValue};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn transaction<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| action().map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

fn event(row: &rusqlite::Row<'_>) -> ServiceResult<Value> {
    let id: i64 = row.get(0).map_err(internal)?;
    let raw: String = row.get(4).map_err(internal)?;
    if raw.len() > MAX_PAGE_BYTES {
        return Err(ServiceError::InvalidInput(format!("Event {id} exceeds the {MAX_PAGE_BYTES}-byte bundle page budget")));
    }
    let data: Value = serde_json::from_str(&raw).map_err(internal)?;
    Ok(json!({"id":id,"timestamp":row.get::<_,String>(1).map_err(internal)?,"type":row.get::<_,String>(2).map_err(internal)?,"instance":row.get::<_,String>(3).map_err(internal)?,"data":data}))
}

/// SQL templates and parameter meanings are private to this fixed capability.
fn page(db: &AgentxDb, predicate: &str, mut parameters: Vec<SqlValue>, cursor: Option<i64>, through: Option<i64>, limit: usize, descending: bool) -> ServiceResult<Page> {
    let through = match through {
        Some(value) => value,
        None => db.conn().query_row("SELECT COALESCE(MAX(id),0) FROM events", [], |row| row.get(0)).map_err(internal)?,
    };
    if through < 0 || cursor.is_some_and(|value| value < 0) {
        return Err(ServiceError::InvalidInput("negative bundle page boundary".into()));
    }
    let limit = limit.min(MAX_PAGE_EVENTS);
    if limit == 0 {
        return Ok(Page { events: Vec::new(), through, next: cursor, has_more: false });
    }
    parameters.push(cursor.map_or(SqlValue::Null, SqlValue::Integer));
    let boundary = parameters.len();
    parameters.push(SqlValue::Integer(through));
    let watermark = parameters.len();
    let (comparison, order) = if descending { ("<", "DESC") } else { (">", "ASC") };
    let sql = format!("SELECT e.id,e.timestamp,e.type,e.instance,e.data FROM events_v e WHERE ({predicate}) AND (?{boundary} IS NULL OR e.id {comparison} ?{boundary}) AND e.id<=?{watermark} ORDER BY e.id {order} LIMIT {}", limit + 1);
    let mut statement = db.conn().prepare(&sql).map_err(internal)?;
    let mut rows = statement.query(rusqlite::params_from_iter(parameters)).map_err(internal)?;
    let mut events = Vec::new();
    let mut bytes = 0;
    let mut has_more = false;
    while let Some(row) = rows.next().map_err(internal)? {
        if events.len() == limit {
            has_more = true;
            break;
        }
        let value = event(row)?;
        let size = serde_json::to_vec(&value).map_err(internal)?.len();
        if bytes + size > MAX_PAGE_BYTES {
            if events.is_empty() {
                return Err(ServiceError::InvalidInput(format!("Event {} exceeds the {MAX_PAGE_BYTES}-byte bundle page budget", value["id"])));
            }
            has_more = true;
            break;
        }
        bytes += size;
        events.push(value);
    }
    let next = events.last().and_then(|event| event["id"].as_i64()).or(cursor);
    Ok(Page { events, through, next, has_more })
}
fn list(db: &AgentxDb, before: Option<i64>, through: Option<i64>, limit: usize) -> ServiceResult<Page> {
    page(db, "e.type='bundle'", Vec::new(), before, through, limit, true)
}
fn context_events(db: &AgentxDb, agent: &str, category: Category, before: Option<i64>, through: Option<i64>, limit: usize) -> ServiceResult<Page> {
    let predicate = match category {
        Category::MessagesTo => "e.type='message' AND EXISTS(SELECT 1 FROM json_each(json_extract(e.data,'$.delivered_to')) WHERE value=?1)".to_owned(),
        Category::MessagesFrom => "e.type='message' AND json_extract(e.data,'$.from')=?1".to_owned(),
        Category::FileOperations => format!("e.type='status' AND e.instance=?1 AND (e.status_context IN {} OR e.status_context LIKE 'tool:%' AND e.status_detail LIKE '/%')", crate::events::filters::FILE_WRITE_CONTEXTS),
        Category::Lifecycle => "e.type='life' AND (e.instance=?1 OR json_extract(e.data,'$.by')=?1)".to_owned(),
    };
    page(db, &predicate, vec![SqlValue::Text(agent.into())], before, through, limit, true)
}
fn referenced(db: &AgentxDb, refs: &[Value], after: Option<i64>, through: Option<i64>, limit: usize) -> ServiceResult<Page> {
    // JSON intervals preserve sparse/huge ranges without allocating every ID or
    // hitting SQLite's parameter/expression-depth limit with an IN/OR expansion.
    let mut ranges = Vec::new();
    for reference in refs {
        let text = reference.as_str().map(str::to_owned).or_else(|| reference.as_i64().map(|id| id.to_string())).unwrap_or_default();
        let bounds = if let Some((start, end)) = text.split_once('-') { start.parse::<i64>().ok().zip(end.parse::<i64>().ok()) } else { text.parse::<i64>().ok().map(|id| (id, id)) };
        if let Some((start, end)) = bounds
            && start > 0
            && end >= start
        {
            ranges.push((start, end));
        }
    }
    let ranges = serde_json::to_string(&ranges).map_err(internal)?;
    page(db, "EXISTS(SELECT 1 FROM json_each(?1) r WHERE e.id BETWEEN json_extract(r.value,'$[0]') AND json_extract(r.value,'$[1]'))", vec![SqlValue::Text(ranges)], after, through, limit, false)
}
fn lookup(db: &AgentxDb, id: &str) -> ServiceResult<Option<Value>> {
    let read = |sql: &str, parameter: SqlValue| -> ServiceResult<Option<Value>> {
        let mut statement = db.conn().prepare(sql).map_err(internal)?;
        let mut rows = statement.query([parameter]).map_err(internal)?;
        rows.next().map_err(internal)?.map(event).transpose()
    };
    let exact = if let Ok(id) = id.parse::<i64>() { read("SELECT id,timestamp,type,instance,data FROM events WHERE type='bundle' AND id=?1", SqlValue::Integer(id))? } else { None };
    let value = if exact.is_some() {
        exact
    } else {
        let prefix = if id.starts_with("bundle:") { id.to_owned() } else { format!("bundle:{id}") };
        // The public operand is a literal prefix, never a LIKE program.
        let pattern = format!("{}%", prefix.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
        read("SELECT id,timestamp,type,instance,data FROM events WHERE type='bundle' AND json_extract(data,'$.bundle_id') LIKE ?1 ESCAPE '\\' ORDER BY id DESC LIMIT 1", SqlValue::Text(pattern))?
    };
    value
        .map(|value| {
            let mut data = value["data"].clone();
            let object = data.as_object_mut().ok_or_else(|| internal("stored bundle must be an object"))?;
            object.insert("event_id".into(), value["id"].clone());
            object.insert("timestamp".into(), value["timestamp"].clone());
            if serde_json::to_vec(&data).map_err(internal)?.len() > MAX_PAGE_BYTES {
                return Err(ServiceError::InvalidInput("bundle exceeds page byte budget".into()));
            }
            Ok(data)
        })
        .transpose()
}

fn unclaimed(invocation: &InvocationContext) -> bool {
    invocation.explicit_name.is_none()
        && invocation.process_binding_id.is_none()
        && invocation.provider_session.is_none()
        && invocation.activation_target.is_none()
        && invocation.actor_capability.is_none()
        && !invocation.environment.contains_key("AGENTX_CLAUDE_ACTOR")
        && !invocation.environment.contains_key("AGENTX_CLAUDE_ACTOR_SESSION")
}
#[derive(serde::Serialize, serde::Deserialize)]
struct RetainedCreate {
    intent: String,
    actor: Option<ParticipantRef>,
    outcome: Created,
}

fn create(db: &AgentxDb, context: &RequestContext, mut bundle: Value, expected: Option<ParticipantRef>) -> ServiceResult<Created> {
    if context.process.pid == 0 || context.process.incarnation.as_deref().is_none_or(str::is_empty) {
        return Err(ServiceError::Forbidden("authenticated bundle caller required".into()));
    }
    if context.caller.request_id.0.is_empty() || context.caller.request_id.0.len() > 128 {
        return Err(ServiceError::InvalidInput("bounded stable request ID required".into()));
    }
    super::bundles::validate_bundle(&mut bundle).map_err(ServiceError::InvalidInput)?;
    if serde_json::to_vec(&bundle).map_err(internal)?.len() > MAX_PAGE_BYTES - 1024 {
        return Err(ServiceError::InvalidInput("bundle exceeds page byte budget".into()));
    }
    let bytes = serde_json::to_vec(&json!({"bundle":bundle,"actor":expected,"invocation":context.caller.invocation,"peer":{"pid":context.process.pid,"incarnation":context.process.incarnation}})).map_err(internal)?;
    let intent = Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    let key = format!("bundle_create:{}:{}", context.database_generation, context.caller.request_id.0);
    transaction(db, || {
        let actor = match crate::agents::service::resolve_actor(db, context) {
            Ok(actor) => Some(actor.participant),
            Err(ServiceError::IdentityRequired(_)) if unclaimed(&context.caller.invocation) => None,
            Err(error) => return Err(error),
        };
        if actor != expected {
            return Err(ServiceError::StaleFence);
        }
        if let Some(stored) = db.kv_get(&key).map_err(internal)? {
            let stored: RetainedCreate = serde_json::from_str(&stored).map_err(internal)?;
            if stored.actor != actor {
                return Err(ServiceError::StaleFence);
            }
            if stored.intent != intent {
                return Err(ServiceError::Conflict("request ID reused for a different bundle or caller".into()));
            }
            let mut outcome = stored.outcome;
            outcome.replayed = true;
            return Ok(outcome);
        }
        let mut warnings = Vec::new();
        if let Some(parent) = bundle.get("extends").and_then(Value::as_str).filter(|value| !value.is_empty()) {
            let search = if parent.starts_with("bundle:") { parent.to_owned() } else { format!("bundle:{parent}") };
            let exists = db.conn().query_row("SELECT 1 FROM events WHERE type='bundle' AND json_extract(data,'$.bundle_id')=?1 LIMIT 1", [search], |_| Ok(())).optional().map_err(internal)?.is_some();
            if !exists {
                warnings.push(format!("Parent bundle not found: {parent}"));
            }
        }
        let bundle_id = bundle.get("bundle_id").and_then(Value::as_str).map(str::to_owned).unwrap_or_else(super::bundles::generate_bundle_id);
        bundle["bundle_id"] = json!(bundle_id);
        let instance = actor.as_ref().map_or("cli", |actor| actor.canonical_name.as_str());
        let object = bundle.as_object_mut().unwrap();
        object.remove("created_by");
        if actor.is_some() {
            object.insert("created_by".into(), json!(instance));
        }
        let event_id = db.log_event("bundle", instance, &bundle).map_err(internal)?;
        let outcome = Created { bundle_id, event_id, warnings, replayed: false };
        let retained = RetainedCreate { intent, actor, outcome: outcome.clone() };
        db.kv_set(&key, Some(&serde_json::to_string(&retained).map_err(internal)?)).map_err(internal)?;
        Ok(outcome)
    })
}

#[cfg(test)]
mod tests;
