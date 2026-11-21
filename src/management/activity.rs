//! Bounded read-only accepted-address observation. First read seeds a watermark.
use super::contract::*;
use crate::agents::attachments::ParticipantRef;
use crate::{
    db::AgentxDb,
    service::{RequestContext, ServiceError, ServiceResult},
};

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn reference(value: ParticipantRef) -> AgentRef {
    AgentRef { name: value.canonical_name, incarnation: Some(value.incarnation_bits.to_string()), stopped_event: None }
}

pub(super) fn read(db: &AgentxDb, context: &RequestContext, after: Option<String>, limit: u32) -> ServiceResult<OfficeActivity> {
    if limit == 0 || limit > MAX_PAGE {
        return Err(ServiceError::InvalidInput("invalid activity page size".into()));
    }
    let watermark = db.get_last_event_id();
    let mailbox = super::human::read(db, context, None, false, 20)?;
    let human_has_more = mailbox.next_before.is_some();
    let human_unhandled_count = mailbox.unhandled_count;
    let human_messages = mailbox.messages;
    let Some(after) = after else {
        return Ok(OfficeActivity { human_messages, human_has_more, human_unhandled_count, generation: context.database_generation.clone(), next_after: watermark.to_string(), has_more: false, messages: vec![], created: vec![] });
    };
    let after = after.parse::<i64>().ok().filter(|value| *value >= 0 && *value <= watermark).ok_or_else(|| ServiceError::InvalidInput("invalid activity cursor".into()))?;
    let mut stmt = db.conn().prepare("SELECT id,timestamp,data FROM events WHERE (type='message' OR (type='life' AND json_extract(data,'$.action')='created')) AND id>?1 AND id<=?2 ORDER BY id LIMIT ?3").map_err(internal)?;
    let rows = stmt.query_map(rusqlite::params![after, watermark, limit + 1], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))).map_err(internal)?.collect::<Result<Vec<_>, _>>().map_err(internal)?;
    let has_more = rows.len() > limit as usize;
    let mut next = after;
    let mut messages = Vec::new();
    let mut created = Vec::new();
    for (id, at, raw) in rows.into_iter().take(limit as usize) {
        next = id;
        let data: serde_json::Value = serde_json::from_str(&raw).map_err(internal)?;
        if data.get("action").and_then(serde_json::Value::as_str) == Some("created") {
            if let Some(value) = data.get("participant").filter(|value| !value.is_null()) {
                created.push(reference(serde_json::from_value(value.clone()).map_err(internal)?));
            }
            continue;
        }
        let Some(saved) = data.get("scene_addressing") else { continue };
        let sender = saved.get("sender").filter(|value| !value.is_null()).map(|value| serde_json::from_value::<ParticipantRef>(value.clone())).transpose().map_err(internal)?.map(reference);
        let recipients = saved.get("recipients").map(|value| serde_json::from_value::<Vec<ParticipantRef>>(value.clone())).transpose().map_err(internal)?.unwrap_or_default().into_iter().map(reference).collect();
        let human_sender = saved.get("human_sender").and_then(serde_json::Value::as_bool).unwrap_or(false);
        let human_recipient = saved.get("human_recipient").and_then(serde_json::Value::as_bool).unwrap_or(false);
        messages.push(OfficeMessage { id: id.to_string(), at, sender, recipients, human_sender, human_recipient });
    }
    Ok(OfficeActivity { human_messages, human_has_more, human_unhandled_count, generation: context.database_generation.clone(), next_after: if has_more { next } else { watermark }.to_string(), has_more, messages, created })
}
