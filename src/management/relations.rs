//! Bounded indexed direct-creation reads and exact current/stopped projections.
use super::contract::*;
use crate::{
    agents::attachments::ParticipantRef,
    db::AgentxDb,
    service::{RequestContext, ServiceError, ServiceResult},
};
use rusqlite::{OptionalExtension, params};
fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn exact(db: &AgentxDb, context: &RequestContext, name: String, incarnation: String, hydrate: bool) -> ServiceResult<RelatedAgent> {
    let launch_failure: Option<String> = db.conn().query_row("SELECT launch_failure FROM agent_creation_instances WHERE name=?1 AND incarnation=?2", params![name, incarnation], |row| row.get(0)).optional().map_err(internal)?.flatten();
    let current = AgentRef { name: name.clone(), incarnation: Some(incarnation.clone()), stopped_event: None };
    let projected = if hydrate { super::store::projected_agent(db, context, &current) } else { super::store::agent(db, &current) };
    let view = match projected {
        Ok(view) => Some(view),
        Err(ServiceError::StaleFence) => {
            let event: Option<i64> = db.conn().query_row("SELECT stopped_event FROM agent_creation_instances WHERE name=?1 AND incarnation=?2", params![name, incarnation], |row| row.get(0)).optional().map_err(internal)?.flatten();
            if let Some(event) = event {
                match super::store::agent(db, &AgentRef { name: name.clone(), incarnation: None, stopped_event: Some(event.to_string()) }) {
                    Ok(view) => Some(view),
                    Err(ServiceError::StaleFence) => None,
                    Err(error) => return Err(error),
                }
            } else {
                None
            }
        }
        Err(error) => return Err(error),
    };
    Ok(RelatedAgent { name, incarnation, launch_failure, availability: if view.is_some() { "available" } else { "unavailable" }.into(), agent: view })
}
pub(crate) fn read(db: &AgentxDb, context: &RequestContext, reference: AgentRef, after: Option<String>, limit: u32) -> ServiceResult<LaunchRelations> {
    if limit == 0 || limit > MAX_PAGE || after.as_ref().is_some_and(|id| uuid::Uuid::parse_str(id).is_err()) {
        return Err(ServiceError::InvalidInput("invalid relation page bounds".into()));
    }
    super::store::agent(db, &reference)?;
    let bits = if let Some(bits) = &reference.incarnation {
        bits.parse::<u64>().map_err(internal)?
    } else {
        let snapshot = super::store::stopped_snapshot(db, &reference)?;
        match snapshot.get("created_at").and_then(serde_json::Value::as_f64) {
            Some(created) => created.to_bits(),
            None => return Ok(LaunchRelations { generation: context.database_generation.clone(), origin: "unknown".into(), upstream: None, fork_source: None, downstream: vec![], next: None }),
        }
    };
    let participant = ParticipantRef { canonical_name: reference.name, incarnation_bits: bits };
    Ok(read_exact(db, context, Some(&participant), after, limit, true)?.1)
}

pub(crate) fn read_exact(db: &AgentxDb, context: &RequestContext, participant: Option<&ParticipantRef>, after: Option<String>, limit: u32, hydrate: bool) -> ServiceResult<(u64, LaunchRelations)> {
    if limit == 0 || limit > MAX_PAGE || after.as_ref().is_some_and(|id| uuid::Uuid::parse_str(id).is_err()) {
        return Err(ServiceError::InvalidInput("invalid relation page bounds".into()));
    }
    let id = participant.map(|participant| crate::agents::launch_relations::creation(db, participant)).transpose().map_err(internal)?.flatten();
    let Some(id) = id else {
        return Ok((0, LaunchRelations { generation: context.database_generation.clone(), origin: "unknown".into(), upstream: None, fork_source: None, downstream: vec![], next: None }));
    };
    // The compact native Info summary shows a count; Web pages need only a cursor.
    let total = if hydrate || after.is_some() { 0 } else { db.conn().query_row("SELECT COUNT(*) FROM agent_creations WHERE creator_creation=?1", [&id], |row| row.get::<_, i64>(0)).map_err(internal)? as u64 };
    if let Some(after) = &after {
        let valid: bool = db.conn().query_row("SELECT EXISTS(SELECT 1 FROM agent_creations WHERE id=?1 AND creator_creation=?2)", params![after, id], |row| row.get(0)).map_err(internal)?;
        if !valid {
            return Err(ServiceError::InvalidInput("relation cursor belongs to another subject".into()));
        }
    }
    let (origin, name, bits): (String, Option<String>, Option<String>) =
        db.conn().query_row("SELECT origin,creator_name,creator_incarnation FROM agent_creations WHERE id=?1", [&id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).map_err(internal)?;
    let upstream = match (name, bits) {
        (Some(name), Some(bits)) => Some(exact(db, context, name, bits, hydrate)?),
        _ => None,
    };
    let fork_source = db
        .conn()
        .query_row("SELECT source_name,source_incarnation,source_stopped_event FROM agent_creation_forks WHERE creation=?1", [&id], |row| {
            Ok(AgentRef { name: row.get(0)?, incarnation: row.get(1)?, stopped_event: row.get::<_, Option<i64>>(2)?.map(|event| event.to_string()) })
        })
        .optional()
        .map_err(internal)?;
    let mut statement = db.conn().prepare("SELECT id,name,incarnation FROM agent_creations WHERE creator_creation=?1 AND id>?2 ORDER BY id LIMIT ?3").map_err(internal)?;
    let rows = statement.query_map(params![id, after.unwrap_or_default(), limit + 1], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))).map_err(internal)?.collect::<Result<Vec<_>, _>>().map_err(internal)?;
    let more = rows.len() > limit as usize;
    let mut next = None;
    let mut downstream = Vec::new();
    for (id, name, bits) in rows.into_iter().take(limit as usize) {
        next = Some(id);
        downstream.push(exact(db, context, name, bits, hydrate)?);
    }
    Ok((total, LaunchRelations { generation: context.database_generation.clone(), origin, upstream, fork_source, downstream, next: if more { next } else { None } }))
}
