//! Exact creation-owned label persistence. SQLite transaction and native fences
//! cover labels, revision, event and retained response as one operation.
use super::{
    attachments::ParticipantRef,
    labels::{LabelOutcome, LabelPage, LabelQuery, LabelSet, LabelSnapshot, MAX_LABELS, PatchLabels, Request, Response, normalize_selectors, validate_key, validate_set},
};
use crate::{
    db::AgentxDb,
    service::{RequestContext, ServiceError, ServiceResult},
};
use rusqlite::{OptionalExtension, params};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn invalid(message: &str) -> ServiceError {
    ServiceError::InvalidInput(message.into())
}
fn read_scope<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    if !db.conn().is_autocommit() {
        return action();
    }
    let transaction = db.conn().unchecked_transaction().map_err(internal)?;
    let result = action()?;
    transaction.commit().map_err(internal)?;
    Ok(result)
}
fn current(db: &AgentxDb, participant: &ParticipantRef) -> ServiceResult<()> {
    let row = db.get_instance_full(&participant.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    if row.created_at.to_bits() != participant.incarnation_bits || row.origin_device_id.is_some() {
        return Err(ServiceError::StaleFence);
    }
    Ok(())
}
fn owner(db: &AgentxDb, participant: &ParticipantRef) -> ServiceResult<Option<String>> {
    super::launch_relations::creation(db, participant).map_err(internal)
}
fn revision(db: &AgentxDb, owner: &str) -> ServiceResult<i64> {
    Ok(db.conn().query_row("SELECT revision FROM agent_label_sets WHERE owner_creation=?", [owner], |row| row.get(0)).optional().map_err(internal)?.unwrap_or(0))
}
fn owner_labels(db: &AgentxDb, owner: &str) -> ServiceResult<LabelSet> {
    let mut statement = db.conn().prepare("SELECT label_key,label_value FROM agent_labels WHERE owner_creation=? ORDER BY label_key").map_err(internal)?;
    statement.query_map([owner], |row| Ok((row.get(0)?, row.get(1)?))).map_err(internal)?.collect::<rusqlite::Result<_>>().map_err(internal)
}

pub(crate) fn get(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef) -> ServiceResult<LabelSnapshot> {
    hydrate(db, context, std::slice::from_ref(participant))?.remove(&participant.canonical_name).ok_or(ServiceError::StaleFence)
}
/// One SQL statement for <=100 exact current targets, including empty sets.
/// No creation mapping is manufactured by this read.
pub(crate) fn hydrate(db: &AgentxDb, context: &RequestContext, participants: &[ParticipantRef]) -> ServiceResult<BTreeMap<String, LabelSnapshot>> {
    if participants.len() > 100 {
        return Err(invalid("label hydration is limited to 100 participants"));
    }
    if participants.iter().map(|p| &p.canonical_name).collect::<BTreeSet<_>>().len() != participants.len() {
        return Err(invalid("duplicate hydration participant"));
    }
    read_scope(db, || {
        let input: Vec<_> = participants.iter().map(|p| json!({"name":p.canonical_name,"bits":p.incarnation_bits.to_string()})).collect();
        let mut statement = db.conn().prepare("WITH requested AS (SELECT json_extract(value,'$.name') AS name,json_extract(value,'$.bits') AS bits FROM json_each(?1)) SELECT r.name,COALESCE(s.revision,0),l.label_key,l.label_value FROM requested r JOIN instances i ON i.name=r.name AND agentx_incarnation(i.created_at)=r.bits AND COALESCE(i.origin_device_id,'')='' LEFT JOIN agent_creation_instances a ON a.name=r.name AND a.incarnation=r.bits LEFT JOIN agent_label_sets s ON s.owner_creation=a.creation LEFT JOIN agent_labels l ON l.owner_creation=a.creation ORDER BY r.name,l.label_key").map_err(internal)?;
        let rows = statement.query_map([serde_json::to_string(&input).map_err(internal)?], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, Option<String>>(3)?))).map_err(internal)?;
        let mut snapshots: BTreeMap<String, LabelSnapshot> = BTreeMap::new();
        for row in rows {
            let (name, revision, key, value) = row.map_err(internal)?;
            let snapshot = snapshots.entry(name.clone()).or_insert_with(|| LabelSnapshot {
                participant: participants.iter().find(|p| p.canonical_name == name).expect("requested").clone(),
                generation: context.database_generation.clone(),
                labels: LabelSet::new(),
                revision: revision.to_string(),
            });
            if let (Some(key), Some(value)) = (key, value) {
                snapshot.labels.insert(key, value);
            }
        }
        if snapshots.len() != participants.len() {
            return Err(ServiceError::StaleFence);
        }
        Ok(snapshots)
    })
}
fn emit(db: &AgentxDb, context: &RequestContext, owner: &str, outcome: &LabelOutcome, set: &LabelSet, remove: &[String]) -> ServiceResult<()> {
    db.log_event("labels_changed", &outcome.snapshot.participant.canonical_name, &json!({"participant":outcome.snapshot.participant,"owner":owner,"generation":context.database_generation,"revision":outcome.snapshot.revision,"set":set,"remove":remove,"request_id":context.caller.request_id,"actor":super::service_store::resolve_actor(db,context).ok().map(|actor| actor.participant)})).map_err(internal)?;
    Ok(())
}
fn patch(db: &AgentxDb, context: &RequestContext, input: PatchLabels) -> ServiceResult<LabelOutcome> {
    validate_set(&input.set)?;
    if input.remove.len() > MAX_LABELS {
        return Err(invalid("at most 64 removals"));
    }
    let mut removals = BTreeSet::new();
    for key in &input.remove {
        validate_key(key)?;
        if !removals.insert(key) || input.set.contains_key(key) {
            return Err(invalid("duplicate or overlapping label key"));
        }
    }
    // Generation and exact target are checked even for retained replies. A
    // namesake successor never receives an old operation's result as its own.
    current(db, &input.participant)?;
    super::service_store::retained(db, context, &json!({"labels_patch":input}), || {
        current(db, &input.participant)?;
        let mut owner_id = owner(db, &input.participant)?;
        let old_revision = owner_id.as_deref().map(|id| revision(db, id)).transpose()?.unwrap_or(0);
        if input.if_revision.as_deref().is_some_and(|expected| expected != old_revision.to_string()) {
            return Err(ServiceError::Conflict("label revision changed; reread this exact target".into()));
        }
        let mut labels = owner_id.as_deref().map(|id| owner_labels(db, id)).transpose()?.unwrap_or_default();
        let mut set = LabelSet::new();
        let mut remove = Vec::new();
        for (key, value) in &input.set {
            if labels.get(key) != Some(value) {
                labels.insert(key.clone(), value.clone());
                set.insert(key.clone(), value.clone());
            }
        }
        for key in &input.remove {
            if labels.remove(key).is_some() {
                remove.push(key.clone());
            }
        }
        validate_set(&labels)?;
        let changed = !set.is_empty() || !remove.is_empty();
        let next_revision = if changed { old_revision.checked_add(1).ok_or_else(|| ServiceError::Conflict("label revision exhausted".into()))? } else { old_revision };
        let outcome = LabelOutcome { snapshot: LabelSnapshot { participant: input.participant.clone(), generation: context.database_generation.clone(), labels, revision: next_revision.to_string() }, changed };
        if changed {
            if owner_id.is_none() {
                super::launch_relations::record(db, context, &input.participant, true)?;
                owner_id = owner(db, &input.participant)?;
            }
            let id = owner_id.as_deref().ok_or_else(|| internal("creation mapping missing"))?;
            db.conn().execute("INSERT INTO agent_label_sets(owner_creation,revision) VALUES (?1,?2) ON CONFLICT(owner_creation) DO UPDATE SET revision=excluded.revision", params![id, next_revision]).map_err(internal)?;
            for (key, value) in &set {
                db.conn().execute("INSERT INTO agent_labels(owner_creation,label_key,label_value) VALUES (?1,?2,?3) ON CONFLICT(owner_creation,label_key) DO UPDATE SET label_value=excluded.label_value", params![id, key, value]).map_err(internal)?;
            }
            for key in &remove {
                db.conn().execute("DELETE FROM agent_labels WHERE owner_creation=?1 AND label_key=?2", params![id, key]).map_err(internal)?;
            }
            emit(db, context, id, &outcome, &set, &remove)?;
        }
        Ok(outcome)
    })
}
/// Called only inside reservation's retained outer transaction. Empty launch
/// labels preserve verified resume state; labels are never copied from a source.
pub(super) fn initialize(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef, labels: &LabelSet) -> ServiceResult<()> {
    validate_set(labels)?;
    if labels.is_empty() {
        return Ok(());
    }
    let id = owner(db, participant)?.ok_or_else(|| internal("reservation creation missing"))?;
    if revision(db, &id)? != 0 || !owner_labels(db, &id)?.is_empty() {
        return Err(ServiceError::Conflict("initial labels cannot replace existing creation labels".into()));
    }
    db.conn().execute("INSERT INTO agent_label_sets(owner_creation,revision) VALUES (?1,1)", [&id]).map_err(internal)?;
    for (key, value) in labels {
        db.conn().execute("INSERT INTO agent_labels(owner_creation,label_key,label_value) VALUES (?1,?2,?3)", params![id, key, value]).map_err(internal)?;
    }
    let outcome = LabelOutcome { snapshot: LabelSnapshot { participant: participant.clone(), generation: context.database_generation.clone(), labels: labels.clone(), revision: "1".into() }, changed: true };
    emit(db, context, &id, &outcome, labels, &[])
}

fn query_error(error: rusqlite::Error) -> ServiceError {
    if error.sqlite_error_code() == Some(rusqlite::ErrorCode::OperationInterrupted) { ServiceError::Unavailable("label selection resource/deadline budget exhausted".into()) } else { internal(error) }
}
pub(crate) fn select_current(db: &AgentxDb, context: &RequestContext, mut query: LabelQuery) -> ServiceResult<LabelPage> {
    use sha2::{Digest, Sha256};
    query.selectors = normalize_selectors(&query.selectors)?;
    if context.caller.deadline_unix_ms.is_some_and(|deadline| deadline <= (crate::shared::time::now_epoch_f64() * 1000.0) as u64) {
        return Err(ServiceError::Unavailable("label selection deadline exhausted".into()));
    }
    if query.limit == 0 || query.limit > 100 || query.after.as_ref().is_some_and(|s| s.len() > 300) || query.workspace.as_ref().is_some_and(|s| s.len() > 4096) || query.directory.as_ref().is_some_and(|s| s.len() > 4096) {
        return Err(invalid("invalid label page bounds"));
    }
    let binding = serde_json::to_vec(&json!({"selectors":query.selectors,"workspace":query.workspace,"include_global":query.include_global,"directory":query.directory,"generation":context.database_generation})).map_err(internal)?;
    let fingerprint: String = Sha256::digest(binding).iter().map(|b| format!("{b:02x}")).collect();
    let after = if let Some(cursor) = &query.after {
        let (bound, name) = cursor.strip_prefix("labels:").and_then(|s| s.split_once(':')).ok_or_else(|| invalid("invalid/current-only label cursor"))?;
        if bound != fingerprint || !super::identity::is_valid_name(name) {
            return Err(invalid("label cursor belongs to another query or generation"));
        }
        name.to_owned()
    } else {
        String::new()
    };
    read_scope(db, || {
        let remaining_ms = context.caller.deadline_unix_ms.map(|deadline| deadline.saturating_sub(crate::shared::time::now_epoch_f64().mul_add(1000.0, 0.0) as u64)).unwrap_or(2000).min(2000);
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(remaining_ms);
        db.with_read_budget(
            move || std::time::Instant::now() >= deadline,
            || {
                let mut records = db.select_labeled_instances(&query, &after).map_err(query_error)?;
                let more = records.len() > query.limit as usize;
                records.truncate(query.limit as usize);
                let participants: Vec<_> = records.iter().map(|row| ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() }).collect();
                let snapshots = hydrate(db, context, &participants)?;
                let next = more.then(|| format!("labels:{fingerprint}:{}", records.last().expect("nonempty page").name));
                Ok(LabelPage { records, snapshots, next })
            },
        )
        .map_err(query_error)?
    })
}
pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    if context.caller.expected_database_generation.as_ref().is_some_and(|expected| expected != &context.database_generation) {
        return Err(ServiceError::StaleFence);
    }
    match request {
        Request::Get(participant) => get(db, context, &participant).map(Response::Snapshot),
        Request::Patch(input) => db.with_write_scope(|| patch(db, context, input).map(Response::Patched).map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error))),
        Request::Select(query) => select_current(db, context, query).map(Response::Page),
    }
}
#[cfg(test)]
mod tests;
