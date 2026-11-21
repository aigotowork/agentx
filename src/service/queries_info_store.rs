//! Resolve one local incarnation or retained stop event before projecting Info.
use super::info::{Projection, Saved, Section, Target, snapshot_creation};
use crate::{
    agents::attachments::ParticipantRef,
    db::AgentxDb,
    service::{RequestContext, ServiceError, ServiceResult},
};
use rusqlite::{OptionalExtension, params};
use serde_json::Value;

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

fn stopped(db: &AgentxDb, name: &str, event: i64, bits: Option<u64>) -> ServiceResult<Saved> {
    let data: String = db
        .conn()
        .query_row("SELECT data FROM events WHERE id=?1 AND instance=?2 AND type='life' AND json_extract(data,'$.action') IN ('stopped','killed')", params![event, name], |row| row.get(0))
        .optional()
        .map_err(internal)?
        .ok_or(ServiceError::StaleFence)?;
    let data: Value = serde_json::from_str(&data).map_err(internal)?;
    let snapshot = data.get("snapshot").filter(|value| value.is_object()).ok_or(ServiceError::StaleFence)?;
    if snapshot.get("origin_device_id").and_then(Value::as_str).is_some_and(|device| !device.is_empty()) || snapshot_creation(snapshot).map(f64::to_bits) != bits {
        return Err(ServiceError::StaleFence);
    }
    Ok(Saved::Stopped(snapshot.clone()))
}

fn resolve(db: &AgentxDb, target: &Target) -> ServiceResult<Saved> {
    match target {
        Target::Stopped { name, event, bits } => stopped(db, name, *event, *bits),
        Target::Incarnation { name, bits } => {
            if let Some(row) = db.get_instance_full(name).map_err(internal)?
                && row.created_at.to_bits() == *bits
                && row.origin_device_id.as_deref().is_none_or(str::is_empty)
            {
                return Ok(Saved::Current(Box::new(row)));
            }
            // An instance can stop while Info is open. Follow only its known exact
            // stop event; never fall back to a later stop or a namesake row.
            let event: Option<i64> = db.conn().query_row("SELECT stopped_event FROM agent_creation_instances WHERE name=?1 AND incarnation=?2", params![name, bits.to_string()], |row| row.get(0)).optional().map_err(internal)?.flatten();
            stopped(db, name, event.ok_or(ServiceError::StaleFence)?, Some(*bits))
        }
    }
}

fn participant(target: &Target) -> Option<ParticipantRef> {
    Some(ParticipantRef { canonical_name: target.name().to_owned(), incarnation_bits: target.bits()? })
}

/// File I/O follows the DB snapshot on the existing binding worker.
pub(crate) fn binding_input(db: &AgentxDb, target: &Target) -> ServiceResult<(Option<ParticipantRef>, Vec<std::path::PathBuf>)> {
    let stored = resolve(db, target)?;
    let workspace = stored.collaboration().and_then(|collaboration| collaboration.workspace);
    let roots = stored.directory().into_iter().map(std::path::PathBuf::from).chain(workspace.map(std::path::PathBuf::from)).filter(|path| path.is_absolute()).collect();
    Ok((participant(target), roots))
}

pub(crate) fn read(db: &AgentxDb, context: &RequestContext, target: &Target, section: Section) -> ServiceResult<Projection> {
    let stored = resolve(db, target)?;
    Ok(match section {
        Section::Facts => {
            let status = match &stored {
                Saved::Current(row) => Some(crate::agents::lifecycle::get_instance_status_stored(row, db).status),
                Saved::Stopped(_) => None,
            };
            Projection::Facts(Box::new(stored.facts(status)))
        }
        Section::Instructions => Projection::Text(stored.collaboration().and_then(|adopted| adopted.role).map(|role| role.prompt)),
        Section::ProfileArguments => Projection::ProfileArguments(stored.profile()),
        Section::Bootstrap => {
            let instructions = participant(target).map(|participant| crate::agents::context_store::read_frozen(db, context, &participant)).transpose()?.flatten();
            Projection::Text(instructions.map(|instructions| instructions.text))
        }
        Section::Relations { after, limit } => {
            let (total_children, page) = crate::management::relations::read_exact(db, context, participant(target).as_ref(), after, limit, false)?;
            Projection::Relations { total_children, page }
        }
        Section::Bindings => return Err(ServiceError::Internal("Agent Info bindings require the file worker".into())),
    })
}
