//! Daemon-owned persistence adapter for native event operations.
use super::service::*;
use crate::db::AgentxDb;
use crate::db::subscriptions::{SubCreateOutcome, build_and_insert_sql_subscription, create_filter_subscription};
use crate::events::filters::{FilterMap, build_sql_from_flags};
use crate::service::{RequestContext, ServiceError, ServiceResult};
use rusqlite::OptionalExtension;
use serde_json::{Value, json};

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

/// Shared observation authority: request IDs may vary between bounded pages,
/// while the authenticated OS peer, invocation and data generation stay fixed.
pub(super) fn peer(context: &RequestContext) -> ServiceResult<String> {
    use sha2::Digest;
    if context.process.pid == 0 || context.process.incarnation.as_deref().is_none_or(str::is_empty) {
        return Err(ServiceError::Forbidden("authenticated observer process required".into()));
    }
    // Correlation changes on every IPC attempt and grants no actor authority.
    let mut invocation = context.caller.invocation.clone();
    invocation.observation_origin = None;
    let value = json!({"pid":context.process.pid,"incarnation":context.process.incarnation,"invocation":invocation});
    Ok(sha2::Sha256::digest(serde_json::to_vec(&value).map_err(internal)?).iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(super) fn transaction<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| action().map_err(anyhow::Error::from)).map_err(|error| error.downcast::<ServiceError>().unwrap_or_else(internal))
}

pub(super) fn predicate(query: &EventQuery) -> ServiceResult<(FilterMap, String)> {
    let filters = query.filters.clone();
    let flags = build_sql_from_flags(&filters).map_err(ServiceError::InvalidInput)?;
    let mut terms = Vec::new();
    if !flags.is_empty() {
        terms.push(format!("({flags})"));
    }
    if let Some(sql) = &query.predicate {
        terms.push(format!("({sql})"));
    }
    if let Some(after) = query.after {
        terms.push(format!("id > {after}"));
    }
    if let Some(through) = query.through {
        terms.push(format!("id <= {through}"));
    }
    Ok((filters, if terms.is_empty() { "1=1".into() } else { terms.join(" AND ") }))
}

/// Read one event-model page. This helper accepts a compiled predicate only
/// inside the daemon; callers can never select a table, statement or SQL args.
fn read_page(conn: &rusqlite::Connection, expression: &str, limit: usize, oldest_first: bool) -> ServiceResult<(Vec<Value>, i64, Vec<String>, bool)> {
    let retired_fields = crate::db::obsolete_instance_columns(conn).map_err(internal)?;
    let limit = limit.min(MAX_PAGE_EVENTS);
    let fetch = if limit == 0 { 0 } else { limit + 1 };
    let order = if oldest_first { "ASC" } else { "DESC" };
    let sql = format!("SELECT id, timestamp, type, instance, data FROM events_v WHERE {expression} ORDER BY id {order} LIMIT {fetch}");
    super::predicate::read(conn, super::predicate::Projection::Events, || read_page_sql(conn, &sql, &[], limit, &retired_fields)).map_err(ServiceError::InvalidInput)
}
fn read_page_sql(conn: &rusqlite::Connection, sql: &str, params: &[&dyn rusqlite::ToSql], limit: usize, retired_fields: &[String]) -> Result<(Vec<Value>, i64, Vec<String>, bool), String> {
    let mut statement = conn.prepare(sql).map_err(|e| format!("Error in SQL WHERE clause: {e}"))?;
    if !statement.readonly() || statement.parameter_count() != params.len() {
        return Err("Event predicates must be read-only and cannot contain unbound parameters".into());
    }
    let mut rows = statement.query(params).map_err(|e| format!("Error in SQL WHERE clause: {e}"))?;
    let mut events = Vec::new();
    let mut through = 0;
    let mut warnings = Vec::new();
    let mut bytes = 0;
    let mut has_more = false;
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        if events.len() >= limit {
            has_more = true;
            break;
        }
        let id = row.get::<_, i64>(0).map_err(|error| error.to_string())?;
        let raw_bytes: usize = (1..=4).map(|index| row.get_ref(index).ok().and_then(|value| value.as_bytes().ok()).map_or(0, |value| value.len())).sum();
        if raw_bytes > MAX_PAGE_BYTES {
            if events.is_empty() {
                return Err(format!("Event {id} exceeds the {MAX_PAGE_BYTES}-byte event page budget"));
            }
            has_more = true;
            break;
        }
        let parsed = (|| -> rusqlite::Result<Value> {
            let data: String = row.get(4)?;
            let event_type: String = row.get(2)?;
            let mut data = serde_json::from_str::<Value>(&data).unwrap_or_else(|_| json!({}));
            AgentxDb::project_event_data(&event_type, &mut data, retired_fields);
            Ok(json!({"id": row.get::<_, i64>(0)?, "ts": row.get::<_, String>(1)?, "type": event_type, "instance": row.get::<_, String>(3)?, "data": data}))
        })();
        match parsed {
            Ok(event) => {
                let size = serde_json::to_vec(&event).map_err(|error| error.to_string())?.len();
                if bytes + size > MAX_PAGE_BYTES {
                    if events.is_empty() {
                        return Err(format!("Event {id} exceeds the {MAX_PAGE_BYTES}-byte event page budget"));
                    }
                    has_more = true;
                    break;
                }
                bytes += size;
                events.push(event);
            }
            Err(error) => warnings.push(format!("Skipping corrupt event {id}: {error}")),
        }
        through = through.max(id);
    }
    Ok((events, through, warnings, has_more))
}

/// Read an exact conversation window with the same page limits and cursor as
/// the general event query. Only complete projections answer from the index.
pub(crate) fn conversation_page(db: &AgentxDb, context: &RequestContext, participant: &str, start: &str, after: i64, through: Option<i64>, limit: usize) -> ServiceResult<EventPage> {
    let upper = through.unwrap_or(i64::MAX);
    if db.conversation_routes_cover(after, upper) {
        // Start from the participant's ordered keys, not from all message rows.
        const SQL: &str = "SELECT e.id,e.timestamp,e.type,e.instance,e.data FROM conversation_routes r
            CROSS JOIN events e ON e.id=r.event_id WHERE r.participant=?1 AND r.event_id>?2 AND r.event_id<=?3
            AND e.type='message' AND e.timestamp>=?4 ORDER BY r.event_id LIMIT ?5";
        let limit = limit.min(MAX_PAGE_EVENTS);
        let fetch = if limit == 0 { 0 } else { (limit + 1) as i64 };
        let (events, next, warnings, has_more) = read_page_sql(db.conn(), SQL, &[&participant, &after, &upper, &start, &fetch], limit, &[]).map_err(ServiceError::InvalidInput)?;
        return Ok(EventPage { events, resolved_filters: FilterMap::new(), newest_id: db.get_last_event_id(), next_after: next.max(after), has_more, database_generation: context.database_generation.clone(), warnings, history: None });
    }
    let filters = FilterMap::from([("type".into(), vec!["message".into()]), ("participant".into(), vec![participant.into()])]);
    query(db, context, EventQuery { filters, predicate: Some(format!("timestamp >= '{start}'")), after: Some(after), through, limit, oldest_first: true, ..Default::default() })
}

/// Descending seek within a caller-pinned snapshot. The returned page is
/// chronological; its first ID can be used as the exclusive older bound.
pub(crate) fn conversation_history(db: &AgentxDb, participant: &str, start: &str, through: i64, limit: usize) -> ServiceResult<(Vec<Value>, bool)> {
    if through <= 0 {
        return Ok((Vec::new(), false));
    }
    const INDEXED: &str = "SELECT e.id,e.timestamp,e.type,e.instance,e.data FROM conversation_routes r
        CROSS JOIN events e ON e.id=r.event_id WHERE r.participant=?1 AND r.event_id>?2 AND r.event_id<=?3
        AND e.type='message' AND e.timestamp>=?4 ORDER BY r.event_id DESC LIMIT ?5";
    const JOURNAL: &str = "SELECT id,timestamp,type,instance,data FROM events NOT INDEXED
        WHERE id>?2 AND id<=?3 AND type='message' AND timestamp>=?4 AND (instance=?1 OR EXISTS (
            SELECT 1 FROM json_each(CASE WHEN json_valid(data) THEN
                CASE WHEN json_valid(json_extract(data,'$.delivered_to')) THEN json_extract(data,'$.delivered_to') END
            END) WHERE json_each.type='text' AND value=?1
        )) ORDER BY id DESC LIMIT ?5";
    const RAW_ROWS: i64 = 4096;
    let (sql, lower) = if db.conversation_routes_cover(0, through) {
        (INDEXED, 0)
    } else {
        // Seek to the first row outside the scan budget. Bound the raw scan
        // by ID rather than materializing payloads or assuming dense IDs.
        let lower = db.conn().query_row("SELECT id FROM events WHERE id>0 AND id<=?1 ORDER BY id DESC LIMIT 1 OFFSET ?2", rusqlite::params![through, RAW_ROWS], |row| row.get::<_, i64>(0)).optional().map_err(internal)?.unwrap_or(0);
        (JOURNAL, lower)
    };
    let limit = limit.min(MAX_PAGE_EVENTS);
    let fetch = (limit + 1) as i64;
    let (mut events, _, _, has_more) = read_page_sql(db.conn(), sql, &[&participant, &lower, &through, &start, &fetch], limit, &[]).map_err(ServiceError::InvalidInput)?;
    if lower != 0 && !has_more {
        return Err(ServiceError::Unavailable("message history scan incomplete while the conversation index is being prepared; retry".into()));
    }
    events.reverse();
    Ok((events, has_more))
}

pub(super) fn query(db: &AgentxDb, context: &RequestContext, query: EventQuery) -> ServiceResult<EventPage> {
    if query.include_archives {
        return super::history_store::query(db, context, query);
    }
    if query.history.is_some() {
        return Err(ServiceError::InvalidInput("history continuation requires archives".into()));
    }
    let (filters, expression) = predicate(&query)?;
    let limit = query.limit.min(MAX_PAGE_EVENTS);
    let (events, through, mut warnings, has_more) = read_page(db.conn(), &expression, limit, query.oldest_first)?;
    if query.warn_unknown_agents {
        for name in filters.get("instance").into_iter().flatten() {
            let seen = db.conn().query_row("SELECT EXISTS(SELECT 1 FROM events WHERE instance=?1)", [name], |row| row.get::<_, bool>(0)).map_err(internal)? || db.get_instance_full(name).map_err(internal)?.is_some();
            if !seen {
                warnings.push(crate::agents::identity::describe_missing_agent(db, name));
            }
        }
    }
    Ok(EventPage { events, resolved_filters: filters, newest_id: db.get_last_event_id(), next_after: through.max(query.after.unwrap_or(0)), has_more, database_generation: context.database_generation.clone(), warnings, history: None })
}

pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    match request {
        Request::Query(request) => query(db, context, request).map(Response::Page),
        Request::Watermark => Ok(Response::Watermark { newest_id: db.get_last_event_id(), database_generation: context.database_generation.clone() }),
        Request::Subscriptions => {
            let values = db.kv_prefix("events_sub:").map_err(internal)?.into_iter().filter_map(|(_, value)| serde_json::from_str(&value).ok()).collect();
            Ok(Response::Subscriptions(values))
        }
        Request::Subscribe { filters, predicate, owner, once, on_hit } => {
            if let Some(expression) = predicate.as_deref() {
                read_page(db.conn(), expression, 0, false)?;
            }
            let parts: Vec<String> = predicate.into_iter().collect();
            let outcome = if filters.is_empty() { build_and_insert_sql_subscription(db, &parts, &owner, once, on_hit.as_deref()) } else { create_filter_subscription(db, &filters, &parts, &owner, once, on_hit.as_deref()) }
                .map_err(ServiceError::InvalidInput)?;
            let created = match outcome {
                SubCreateOutcome::AlreadyExists { id } => SubscriptionCreated { id, already_exists: true, predicate: None, historical_matches: None, latest_match: None },
                SubCreateOutcome::Created { id, final_sql } => {
                    let count =
                        super::predicate::read(db.conn(), super::predicate::Projection::Events, || db.conn().query_row(&format!("SELECT COUNT(*) FROM events_v WHERE ({final_sql})"), [], |row| row.get::<_, i64>(0)).map_err(|error| error.to_string()))
                            .ok();
                    let latest = read_page(db.conn(), &format!("({final_sql})"), 1, false).ok().and_then(|(events, _, _, _)| events.into_iter().next());
                    SubscriptionCreated { id, already_exists: false, predicate: Some(final_sql), historical_matches: count, latest_match: latest }
                }
            };
            Ok(Response::Subscribed(created))
        }
        Request::Unsubscribe { id } => {
            let key = format!("events_sub:{id}");
            let exists = db.kv_get(&key).map_err(internal)?.is_some();
            if exists {
                db.kv_set(&key, None).map_err(internal)?;
            }
            Ok(Response::Unsubscribed(exists))
        }
        Request::BeginListen(spec) => super::listening_store::begin(db, context, spec).map(Response::ListenReady),
        Request::PulseListen { participant, id } => super::listening_store::pulse(db, context, &participant, &id).map(|connected| Response::ListenState { connected }),
        Request::EndListen { participant, id, reason } => super::listening_store::end(db, context, &participant, &id, reason).map(|()| Response::ListenEnded),
        Request::BeginObserver(spec) => super::observer_store::begin(db, context, spec).map(|()| Response::ObserverReady),
        Request::CheckObserver(spec) => super::observer_store::check(db, context, &spec).map(|()| Response::ObserverReady),
        Request::EndObserver(spec) => super::observer_store::end(db, context, &spec).map(|()| Response::ObserverEnded),
        Request::EndHistory { snapshot } => super::history_store::end(db, context, &snapshot).map(|()| Response::HistoryEnded),
    }
}
