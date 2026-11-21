//! Immutable AgentX creation facts and exact resume aliases. Provider ancestry
//! never enters this model. Mutators compose with the caller's write scope.
use super::attachments::ParticipantRef;
use crate::{
    db::AgentxDb,
    service::{RequestContext, ServiceError, ServiceResult},
};
use rusqlite::{OptionalExtension, params};

/// Internal creation policy selected by the fenced lifecycle operation.
pub(super) enum Creation {
    New,
    Resume(Option<ParticipantRef>),
    Fork(Option<ForkSource>),
}

/// Preserve the actual fenced input, including a stopped event when selected.
pub(super) enum ForkSource {
    Current(ParticipantRef),
    Stopped { name: String, event: i64 },
}
pub(super) fn apply(db: &AgentxDb, context: &RequestContext, target: &ParticipantRef, policy: Creation) -> ServiceResult<()> {
    match policy {
        Creation::New => record(db, context, target, false),
        Creation::Resume(Some(source)) => inherit(db, target, &source).map_err(internal),
        Creation::Resume(None) => record(db, context, target, true),
        Creation::Fork(source) => {
            record(db, context, target, false)?;
            if let Some(source) = source {
                let (name, incarnation, event) = match source {
                    ForkSource::Current(source) => (source.canonical_name, Some(source.incarnation_bits.to_string()), None),
                    ForkSource::Stopped { name, event } => (name, None, Some(event)),
                };
                let id = creation(db, target).map_err(internal)?.ok_or_else(|| internal("missing fork creation"))?;
                db.conn().execute(
                    "INSERT INTO agent_creation_forks(creation,source_name,source_incarnation,source_stopped_event) VALUES (?1,?2,?3,?4)",
                    params![id, name, incarnation, event],
                ).map_err(internal)?;
            }
            Ok(())
        }
    }
}
fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
pub(crate) fn creation(db: &AgentxDb, participant: &ParticipantRef) -> anyhow::Result<Option<String>> {
    Ok(db.conn().query_row("SELECT creation FROM agent_creation_instances WHERE name=?1 AND incarnation=?2", params![participant.canonical_name, participant.incarnation_bits.to_string()], |row| row.get(0)).optional()?)
}
/// Unknown legacy actors receive an alias so their future launches can be indexed;
/// this does not manufacture their own historical creator.
pub(crate) fn record(db: &AgentxDb, context: &RequestContext, target: &ParticipantRef, unknown: bool) -> ServiceResult<()> {
    if creation(db, target).map_err(internal)?.is_some() {
        return Ok(());
    }
    let unknown = unknown || context.caller.invocation.environment.contains_key("AGENTX_LAUNCH_ORIGIN_UNAVAILABLE");
    let actor = (!unknown).then(|| super::service_store::resolve_actor(db, context)).and_then(Result::ok).filter(|actor| actor.participant != *target);
    let claimed = context.caller.invocation.process_binding_id.is_some()
        || context.caller.invocation.provider_session.is_some()
        || context.caller.invocation.activation_target.is_some()
        || context.caller.invocation.explicit_name.is_some()
        || context.caller.invocation.actor_capability.is_some();
    let origin = if actor.is_some() {
        "agent"
    } else if unknown || claimed {
        "unknown"
    } else {
        "external"
    };
    let creator = if let Some(actor) = &actor {
        if creation(db, &actor.participant).map_err(internal)?.is_none() {
            insert(db, &actor.participant, "unknown", None, None).map_err(internal)?;
        }
        creation(db, &actor.participant).map_err(internal)?
    } else {
        None
    };
    insert(db, target, origin, actor.as_ref().map(|actor| &actor.participant), creator.as_deref()).map_err(internal)
}
fn insert(db: &AgentxDb, target: &ParticipantRef, origin: &str, creator: Option<&ParticipantRef>, creator_creation: Option<&str>) -> anyhow::Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    db.conn().execute(
        "INSERT INTO agent_creations(id,name,incarnation,origin,creator_name,creator_incarnation,creator_creation) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![id, target.canonical_name, target.incarnation_bits.to_string(), origin, creator.map(|p| p.canonical_name.as_str()), creator.map(|p| p.incarnation_bits.to_string()), creator_creation],
    )?;
    db.conn().execute("INSERT INTO agent_creation_instances(name,incarnation,creation) VALUES (?1,?2,?3)", params![target.canonical_name, target.incarnation_bits.to_string(), id])?;
    Ok(())
}
/// A native failed spawn remains observable if cleanup removes its conversation.
pub(super) fn failed(db: &AgentxDb, target: &ParticipantRef) -> ServiceResult<()> {
    db.conn().execute("UPDATE agent_creation_instances SET launch_failure='cancelled_before_ready' WHERE name=?1 AND incarnation=?2", params![target.canonical_name, target.incarnation_bits.to_string()]).map_err(internal)?;
    Ok(())
}
pub(crate) fn inherit(db: &AgentxDb, target: &ParticipantRef, source: &ParticipantRef) -> anyhow::Result<()> {
    if creation(db, source)?.is_none() {
        insert(db, source, "unknown", None, None)?;
    }
    let id = creation(db, source)?.expect("inserted source");
    if let Some(existing) = creation(db, target)? {
        anyhow::ensure!(existing == id, "cannot reparent an existing creation");
        return Ok(());
    }
    db.conn().execute("INSERT INTO agent_creation_instances(name,incarnation,creation) VALUES (?1,?2,?3) ", params![target.canonical_name, target.incarnation_bits.to_string(), id])?;
    Ok(())
}

#[cfg(test)]
mod tests;
