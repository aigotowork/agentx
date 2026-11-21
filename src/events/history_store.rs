//! Fixed archive/live history windows. Only the daemon opens archive databases;
//! a small keyset cursor refers to a retained, peer-bound source manifest.

use super::service::{EventPage, EventQuery, HistoryCursor, HistoryPosition, MAX_PAGE_BYTES, MAX_PAGE_EVENTS};
use crate::db::AgentxDb;
use crate::service::{RequestContext, ServiceError, ServiceResult};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct Source {
    name: String,
    through: i64,
    file_id: u64,
    generation: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Snapshot {
    peer: String,
    intent: Value,
    filters: crate::events::filters::FilterMap,
    expression: String,
    sources: Vec<Source>,
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn invalid(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::InvalidInput(error.to_string())
}
fn key(context: &RequestContext, id: &str) -> String {
    format!("event_history:{}:{id}", context.database_generation)
}
fn intent(query: &EventQuery) -> Value {
    json!({"filters":query.filters,"predicate":query.predicate,"after":query.after,"through":query.through,"oldest_first":query.oldest_first})
}
fn watermark(conn: &Connection) -> ServiceResult<i64> {
    conn.query_row("SELECT COALESCE(MAX(id),0) FROM events", [], |row| row.get(0)).map_err(internal)
}
fn generation(conn: &Connection) -> ServiceResult<Option<String>> {
    // Pre-daemon archives may have no generation key; never migrate them.
    conn.query_row("SELECT value FROM kv WHERE key='service:database_generation'", [], |row| row.get(0)).optional().map_err(internal)
}
fn archive_path(db: &AgentxDb, name: &str) -> PathBuf {
    db.path().parent().expect("managed database has a directory").join("archive").join(name).join("agentx.db")
}
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
fn history_order(a: &Value, b: &Value) -> std::cmp::Ordering {
    a["ts"].as_str().cmp(&b["ts"].as_str()).then_with(|| a["source"].as_str().cmp(&b["source"].as_str())).then_with(|| a["id"].as_i64().cmp(&b["id"].as_i64()))
}

fn capture(db: &AgentxDb, context: &RequestContext, query: &EventQuery) -> ServiceResult<(Snapshot, Vec<String>)> {
    let (filters, expression) = super::store::predicate(query)?;
    let mut sources = vec![Source { name: "current".into(), through: watermark(db.conn())?, file_id: 0, generation: Some(context.database_generation.clone()) }];
    let mut warnings = Vec::new();
    let root = archive_path(db, "").parent().unwrap().to_path_buf();
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => Some(entries),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(internal(error)),
    };
    for entry in entries.into_iter().flatten() {
        let entry = entry.map_err(internal)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("session-") || !entry.file_type().map_err(internal)?.is_dir() {
            continue;
        }
        let captured = (|| {
            let path = archive_path(db, &name);
            let file_id = crate::sys::fs::file_id(&path);
            if file_id == 0 {
                return Err(internal("archive identity unavailable"));
            }
            let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(internal)?;
            let source = Source { name: name.clone(), through: watermark(&conn)?, file_id, generation: generation(&conn)? };
            // Validate the same event predicate against this source now. The
            // manifest never later introduces a source skipped with a warning.
            read_source(&conn, &source, &expression, None, 0, query.oldest_first)?;
            if crate::sys::fs::file_id(&path) != file_id {
                return Err(ServiceError::StaleFence);
            }
            Ok(source)
        })();
        match captured {
            Ok(source) => sources.push(source),
            Err(error) => {
                if warnings.len() < 20 {
                    warnings.push(format!("Cannot query archive {name}: {error}"));
                }
            }
        }
    }
    sources.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((Snapshot { peer: super::store::peer(context)?, intent: intent(query), filters, expression, sources }, warnings))
}

/// At most one source page is held alongside the globally retained page. ID
/// fences exclude appends, and tuple comparison preserves ties across sources.
fn read_source(conn: &Connection, source: &Source, expression: &str, boundary: Option<&HistoryPosition>, limit: usize, ascending: bool) -> ServiceResult<(Vec<Value>, bool)> {
    let retired_fields = crate::db::obsolete_instance_columns(conn).map_err(internal)?;
    let mut expression = format!("({expression}) AND id <= {}", source.through);
    if let Some(position) = boundary {
        let comparison = if ascending { ">" } else { "<" };
        expression.push_str(&format!(" AND (timestamp,{},id) {comparison} ({},{},{})", quote(&source.name), quote(&position.timestamp), quote(&position.source), position.id));
    }
    let order = if ascending { "ASC" } else { "DESC" };
    let fetch = if limit == 0 { 0 } else { limit + 1 };
    // Structured collision filters and normal event subqueries see the same
    // source watermark as the outer row, including after a later append.
    let sql = format!(
        "WITH events AS MATERIALIZED (SELECT * FROM main.events WHERE id <= {through}), events_v AS MATERIALIZED (SELECT * FROM main.events_v WHERE id <= {through}) SELECT id,timestamp,type,instance,data FROM events_v WHERE {expression} ORDER BY timestamp {order},id {order} LIMIT {fetch}",
        through = source.through
    );
    super::predicate::read(conn, super::predicate::Projection::Events, || {
        let mut statement = conn.prepare(&sql).map_err(|e| format!("Error in SQL WHERE clause: {e}"))?;
        if !statement.readonly() || statement.parameter_count() != 0 {
            return Err("Event predicates must be read-only and cannot contain unbound parameters".into());
        }
        let mut rows = statement.query([]).map_err(|e| e.to_string())?;
        let mut events = Vec::new();
        let mut bytes = 0;
        let mut more = false;
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            if events.len() == limit {
                more = true;
                break;
            }
            let id: i64 = row.get(0).map_err(|e| e.to_string())?;
            let data: String = row.get(4).map_err(|e| e.to_string())?;
            if data.len() > MAX_PAGE_BYTES {
                if events.is_empty() {
                    return Err(format!("Event {id} exceeds the {MAX_PAGE_BYTES}-byte event page budget"));
                }
                more = true;
                break;
            }
            let event_type: String = row.get(2).map_err(|e| e.to_string())?;
            let mut data = serde_json::from_str::<Value>(&data).unwrap_or_else(|_| json!({}));
            AgentxDb::project_event_data(&event_type, &mut data, &retired_fields);
            let event = json!({"id":id,"ts":row.get::<_,String>(1).map_err(|e|e.to_string())?,"type":event_type,"instance":row.get::<_,String>(3).map_err(|e|e.to_string())?,"data":data,"source":source.name});
            let size = serde_json::to_vec(&event).map_err(|e| e.to_string())?.len();
            if bytes + size > MAX_PAGE_BYTES {
                if events.is_empty() {
                    return Err(format!("Event {id} exceeds the {MAX_PAGE_BYTES}-byte event page budget"));
                }
                more = true;
                break;
            }
            bytes += size;
            events.push(event);
        }
        Ok((events, more))
    })
    .map_err(invalid)
}

pub(super) fn query(db: &AgentxDb, context: &RequestContext, query: EventQuery) -> ServiceResult<EventPage> {
    super::store::transaction(db, || {
        let (id, snapshot, warnings, is_new) = if let Some(cursor) = &query.history {
            let raw = db.kv_get(&key(context, &cursor.snapshot)).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
            let snapshot: Snapshot = serde_json::from_str(&raw).map_err(internal)?;
            if snapshot.peer != super::store::peer(context)? {
                return Err(ServiceError::StaleFence);
            }
            if snapshot.intent != intent(&query) {
                return Err(ServiceError::Conflict("history continuation changed its filters, predicate or ordering".into()));
            }
            if !snapshot.sources.iter().any(|source| source.name == cursor.position.source) {
                return Err(ServiceError::InvalidInput("history position has an unknown source".into()));
            }
            (cursor.snapshot.clone(), snapshot, Vec::new(), false)
        } else {
            let (snapshot, warnings) = capture(db, context, &query)?;
            (format!("history-{}", uuid::Uuid::new_v4()), snapshot, warnings, true)
        };
        let limit = query.limit.min(MAX_PAGE_EVENTS);
        let boundary = query.history.as_ref().map(|cursor| &cursor.position);
        let mut events = Vec::new();
        let mut has_more = false;
        let mut newest_id = 0;
        for source in &snapshot.sources {
            let (batch, more) = if source.name == "current" {
                if source.generation.as_deref() != Some(context.database_generation.as_str()) {
                    return Err(ServiceError::StaleFence);
                }
                newest_id = source.through;
                read_source(db.conn(), source, &snapshot.expression, boundary, limit, query.oldest_first)?
            } else {
                let path = archive_path(db, &source.name);
                if crate::sys::fs::file_id(&path) != source.file_id {
                    return Err(ServiceError::StaleFence);
                }
                let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(internal)?;
                if generation(&conn)? != source.generation {
                    return Err(ServiceError::StaleFence);
                }
                let page = read_source(&conn, source, &snapshot.expression, boundary, limit, query.oldest_first)?;
                if crate::sys::fs::file_id(&path) != source.file_id {
                    return Err(ServiceError::StaleFence);
                }
                page
            };
            has_more |= more;
            events.extend(batch);
            events.sort_by(|a, b| if query.oldest_first { history_order(a, b) } else { history_order(b, a) });
            has_more |= events.len() > limit;
            events.truncate(limit);
            let mut bytes = 0;
            let keep = events
                .iter()
                .take_while(|event| {
                    bytes += serde_json::to_vec(event).map_or(MAX_PAGE_BYTES + 1, |value| value.len());
                    bytes <= MAX_PAGE_BYTES
                })
                .count();
            has_more |= events.len() > keep;
            events.truncate(keep);
        }
        let position = events.last().map(|event| HistoryPosition { timestamp: event["ts"].as_str().unwrap().into(), source: event["source"].as_str().unwrap().into(), id: event["id"].as_i64().unwrap() });
        let history = if let Some(position) = position.filter(|_| has_more || !is_new) {
            if is_new {
                db.kv_set(&key(context, &id), Some(&serde_json::to_string(&snapshot).map_err(internal)?)).map_err(internal)?;
            }
            Some(HistoryCursor { snapshot: id, position })
        } else {
            None
        };
        if has_more && history.is_none() {
            return Err(internal("history page has no continuation boundary"));
        }
        let next_after = events.iter().filter(|event| event["source"] == "current").filter_map(|event| event["id"].as_i64()).max().unwrap_or(query.after.unwrap_or(0));
        Ok(EventPage { events, resolved_filters: snapshot.filters, newest_id, next_after, has_more, database_generation: context.database_generation.clone(), warnings, history })
    })
}

pub(super) fn end(db: &AgentxDb, context: &RequestContext, id: &str) -> ServiceResult<()> {
    super::store::transaction(db, || {
        if let Some(raw) = db.kv_get(&key(context, id)).map_err(internal)? {
            let snapshot: Snapshot = serde_json::from_str(&raw).map_err(internal)?;
            if snapshot.peer != super::store::peer(context)? {
                return Err(ServiceError::StaleFence);
            }
            db.kv_set(&key(context, id), None).map_err(internal)?;
        }
        Ok(())
    })
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
