//! Scope-human handling, not Agent delivery or proof of reading. Events remain immutable.
use super::contract::{HumanMailbox, HumanMessage, MAX_PAGE};
use crate::{
    db::{AgentxDb, HUMAN_ADDRESSED},
    service::{RequestContext, ServiceError, ServiceResult},
};

const FIELDS: &str = "e.id,e.timestamp,json_extract(e.data,'$.from'),json_extract(e.data,'$.text'),h.value,json_extract(e.data,'$.scene_addressing.sender')";
fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn invalid(reason: &str) -> ServiceError {
    ServiceError::InvalidInput(reason.into())
}
fn prefix(context: &RequestContext) -> String {
    format!("management:human-handled:{}:", context.database_generation)
}
fn event_id(value: &str) -> ServiceResult<i64> {
    value.parse::<i64>().ok().filter(|id| *id > 0 && id.to_string() == value).ok_or_else(|| invalid("invalid human message ID"))
}

pub(super) fn read(db: &AgentxDb, context: &RequestContext, before: Option<String>, pending: bool, limit: u32) -> ServiceResult<HumanMailbox> {
    if db.read_database_generation().map_err(internal)? != context.database_generation {
        return Err(ServiceError::StaleFence);
    }
    if limit == 0 || limit > MAX_PAGE {
        return Err(invalid("invalid mailbox page size"));
    }
    let before = before.as_deref().map(event_id).transpose()?;
    let prefix = prefix(context);
    read_page(db, context, &prefix, before, pending, limit, db.human_mailbox_covered()).map_err(internal)
}

fn read_page(db: &AgentxDb, context: &RequestContext, prefix: &str, before: Option<i64>, pending: bool, limit: u32, indexed: bool) -> rusqlite::Result<HumanMailbox> {
    let (source, id, addressed) = if indexed { ("human_mailbox_events m", "m.event_id", "1") } else { ("events e", "e.id", HUMAN_ADDRESSED) };
    let join = format!("LEFT JOIN kv h ON h.key=?1 || {id}");
    let count = format!("SELECT COUNT(*) FROM {source} {join} WHERE {addressed} AND h.value IS NULL");
    let unhandled_count = db.conn().prepare_cached(&count)?.query_row([prefix], |row| row.get::<_, i64>(0))? as u64;
    let cursor = if before.is_some() { format!("{id}<?2") } else { "?2 IS NULL".into() };
    let page = format!("SELECT {id} AS id,h.value FROM {source} {join} WHERE {addressed} AND {cursor} AND (?3=0 OR h.value IS NULL) ORDER BY {id} DESC LIMIT ?4");
    let sql = format!("SELECT {FIELDS} FROM ({page}) h JOIN events e ON e.id=h.id ORDER BY e.id DESC");
    let mut statement = db.conn().prepare_cached(&sql)?;
    let mut messages = statement
        .query_map(rusqlite::params![prefix, before, pending, limit + 1], |row| {
            Ok(HumanMessage {
                id: row.get::<_, i64>(0)?.to_string(),
                at: row.get(1)?,
                from: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                text: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                handled_at: row.get(4)?,
                sender: row
                    .get::<_, Option<String>>(5)?
                    .map(|value| {
                        serde_json::from_str::<crate::agents::attachments::ParticipantRef>(&value)
                            .map(|sender| super::contract::AgentRef { name: sender.canonical_name, incarnation: Some(sender.incarnation_bits.to_string()), stopped_event: None })
                            .map_err(|error| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(error)))
                    })
                    .transpose()?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let has_more = messages.len() > limit as usize;
    messages.truncate(limit as usize);
    let next_before = has_more.then(|| messages.last().expect("nonempty bounded page").id.clone());
    Ok(HumanMailbox { generation: context.database_generation.clone(), messages, next_before, unhandled_count })
}

pub(super) fn handle(db: &AgentxDb, context: &RequestContext, ids: Vec<String>) -> ServiceResult<HumanMailbox> {
    if ids.is_empty() || ids.len() > MAX_PAGE as usize {
        return Err(invalid("supply 1..100 explicit human message IDs"));
    }
    let ids = ids.iter().map(|id| event_id(id)).collect::<ServiceResult<Vec<_>>>()?;
    db.with_write_scope(|| {
        if db.read_database_generation().map_err(internal)? != context.database_generation {
            return Err(anyhow::Error::new(ServiceError::StaleFence));
        }
        for id in &ids {
            let exists = db.conn().prepare_cached(&format!("SELECT EXISTS(SELECT 1 FROM events e WHERE e.id=?1 AND {HUMAN_ADDRESSED})")).map_err(internal)?.query_row([id], |row| row.get::<_, bool>(0)).map_err(internal)?;
            if !exists {
                return Err(anyhow::Error::new(invalid("message is not addressed to bigboss")));
            }
        }
        let at = chrono::Utc::now().to_rfc3339();
        for id in ids {
            // First mark wins, including concurrent/idempotent retries. No cursor or event deletion.
            db.conn().execute("INSERT OR IGNORE INTO kv(key,value) VALUES (?1,?2)", rusqlite::params![format!("{}{id}", prefix(context)), at]).map_err(internal)?;
        }
        read(db, context, None, false, 20).map_err(anyhow::Error::new)
    })
    .map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}
