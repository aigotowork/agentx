//! Daemon-owned listener lifetime. A retained begin never recreates a fired
//! one-shot subscription; cleanup cannot remove a newer listener's endpoint.

use super::service::{EventQuery, ListenEnd, ListenReady, ListenSpec};
use crate::agents::{attachments::ParticipantRef, service::resolve_actor};
use crate::db::{AgentxDb, InstanceRow};
use crate::service::{RequestContext, ServiceError, ServiceResult};
use serde::{Deserialize, Serialize};
use sha2::Digest;

#[derive(Serialize, Deserialize)]
struct Authority {
    spec: ListenSpec,
    intent: String,
    peer: String,
    ready: ListenReady,
    ended: bool,
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn key(context: &RequestContext, id: &str) -> String {
    format!("event_listener:{}:{id}", context.database_generation)
}
fn owner(participant: &ParticipantRef) -> String {
    format!("event_listener_owner:{}:{}", participant.canonical_name, participant.incarnation_bits)
}
fn endpoint_owner(participant: &ParticipantRef, filtered: bool) -> String {
    format!("event_listener_endpoint:{}:{}:{filtered}", participant.canonical_name, participant.incarnation_bits)
}
fn transaction<T>(db: &AgentxDb, action: impl FnOnce() -> ServiceResult<T>) -> ServiceResult<T> {
    db.with_write_scope(|| action().map_err(anyhow::Error::from)).map_err(|error| error.downcast::<ServiceError>().unwrap_or_else(internal))
}
fn hash(value: &impl Serialize) -> ServiceResult<String> {
    let normalized = serde_json::to_value(value).map_err(internal)?;
    Ok(sha2::Sha256::digest(serde_json::to_vec(&normalized).map_err(internal)?).iter().map(|byte| format!("{byte:02x}")).collect())
}
fn peer(context: &RequestContext) -> ServiceResult<String> {
    super::store::peer(context)
}
fn current(db: &AgentxDb, participant: &ParticipantRef) -> ServiceResult<Option<InstanceRow>> {
    let row = db.get_instance_full(&participant.canonical_name).map_err(internal)?;
    if row.as_ref().is_some_and(|row| row.created_at.to_bits() != participant.incarnation_bits) {
        return Err(ServiceError::StaleFence);
    }
    Ok(row)
}
fn authenticate(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef) -> ServiceResult<()> {
    if resolve_actor(db, context)?.participant != *participant {
        return Err(ServiceError::StaleFence);
    }
    Ok(())
}
fn load(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef, id: &str) -> ServiceResult<Authority> {
    let value = db.kv_get(&key(context, id)).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    let authority: Authority = serde_json::from_str(&value).map_err(internal)?;
    if authority.spec.participant != *participant || authority.peer != peer(context)? {
        return Err(ServiceError::StaleFence);
    }
    Ok(authority)
}
fn save(db: &AgentxDb, context: &RequestContext, authority: &Authority) -> ServiceResult<()> {
    db.kv_set(&key(context, &authority.spec.id), Some(&serde_json::to_string(authority).map_err(internal)?)).map_err(internal)
}
fn heartbeat(db: &AgentxDb, participant: &ParticipantRef) -> ServiceResult<()> {
    // Listen is not a hook TCP wait. Never modify tcp_mode or wait_timeout.
    db.conn().execute("UPDATE instances SET last_stop=? WHERE name=?", rusqlite::params![crate::shared::time::now_epoch_i64(), participant.canonical_name]).map_err(internal)?;
    Ok(())
}
fn status(db: &AgentxDb, row: &InstanceRow, state: &str, context: &str) -> ServiceResult<()> {
    if row.tool == "adhoc" {
        crate::agents::lifecycle::try_set_status(db, &row.name, state, context, Default::default()).map_err(internal)?;
    }
    Ok(())
}

pub(super) fn begin(db: &AgentxDb, context: &RequestContext, spec: ListenSpec) -> ServiceResult<ListenReady> {
    let suffix = spec.id.strip_prefix("listen-").ok_or_else(|| ServiceError::InvalidInput("invalid listener ID".into()))?;
    if uuid::Uuid::parse_str(suffix).is_err() || spec.port == Some(0) {
        return Err(ServiceError::InvalidInput("invalid listener endpoint or ID".into()));
    }
    let peer = peer(context)?;
    let intent = hash(&spec)?;
    transaction(db, || {
        let row = current(db, &spec.participant)?.ok_or(ServiceError::StaleFence)?;
        authenticate(db, context, &spec.participant)?;
        if db.kv_get(&key(context, &spec.id)).map_err(internal)?.is_some() {
            let authority = load(db, context, &spec.participant, &spec.id)?;
            if authority.intent != intent {
                return Err(ServiceError::Conflict("listener ID reused for another intent".into()));
            }
            // Returning the frozen result must not install the subscription again.
            if !authority.ended && db.kv_get(&owner(&spec.participant)).map_err(internal)?.as_deref() != Some(&spec.id) {
                return Err(ServiceError::StaleFence);
            }
            return Ok(authority.ready);
        }
        let filtered = !spec.filters.is_empty() || spec.predicate.as_ref().is_some_and(|value| !value.is_empty());
        let mut recent = None;
        let expression = if filtered {
            let query = EventQuery { filters: spec.filters.clone(), predicate: spec.predicate.clone(), limit: 1, ..Default::default() };
            let (_, expression) = super::store::predicate(&query)?;
            let cutoff = chrono::DateTime::from_timestamp(crate::shared::time::now_epoch_i64() - 10, 0).map(|time| time.format("%Y-%m-%dT%H:%M:%S").to_string()).unwrap_or_default();
            let query = EventQuery { predicate: Some(format!("({expression}) AND timestamp > '{cutoff}'")), ..query };
            recent = super::store::query(db, context, query)?.events.into_iter().next();
            Some(expression)
        } else {
            None
        };
        let ended = recent.is_some();
        if !ended {
            let status_context = if filtered { format!("filter:{}", spec.id) } else { "ready".into() };
            status(db, &row, "listening", &status_context)?;
            if let Some(expression) = expression {
                let last_id = db.conn().query_row("SELECT COALESCE(MAX(id),0) FROM events", [], |row| row.get(0)).map_err(internal)?;
                crate::db::subscriptions::insert_listen_subscription(db, &spec.id, &spec.participant, &expression, last_id).map_err(internal)?;
            }
            if let Some(port) = spec.port {
                db.upsert_notify_endpoint(&row.name, if filtered { "listen_filter" } else { "listen" }, port).map_err(internal)?;
                db.kv_set(&endpoint_owner(&spec.participant, filtered), Some(&spec.id)).map_err(internal)?;
            }
            heartbeat(db, &spec.participant)?;
            db.kv_set(&owner(&spec.participant), Some(&spec.id)).map_err(internal)?;
        }
        let ready = ListenReady { recent, filtered };
        save(db, context, &Authority { spec, intent, peer, ready: ready.clone(), ended })?;
        Ok(ready)
    })
}

pub(super) fn pulse(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef, id: &str) -> ServiceResult<bool> {
    transaction(db, || {
        let authority = load(db, context, participant, id)?;
        let Some(_) = current(db, participant)? else {
            return Ok(false);
        };
        authenticate(db, context, participant)?;
        if authority.ended || db.kv_get(&owner(participant)).map_err(internal)?.as_deref() != Some(id) {
            return Err(ServiceError::StaleFence);
        }
        heartbeat(db, participant)?;
        Ok(true)
    })
}

pub(super) fn end(db: &AgentxDb, context: &RequestContext, participant: &ParticipantRef, id: &str, reason: Option<ListenEnd>) -> ServiceResult<()> {
    transaction(db, || {
        let mut authority = load(db, context, participant, id)?;
        let row = current(db, participant)?;
        if row.is_some() {
            authenticate(db, context, participant)?;
        }
        if authority.ended {
            return Ok(());
        }
        if authority.ready.filtered {
            crate::db::subscriptions::remove_listen_subscription(db, id, participant).map_err(internal)?;
        }
        if let Some(port) = authority.spec.port
            && db.kv_get(&endpoint_owner(participant, authority.ready.filtered)).map_err(internal)?.as_deref() == Some(id)
        {
            db.conn().execute("DELETE FROM notify_endpoints WHERE instance=? AND kind=? AND port=?", rusqlite::params![participant.canonical_name, if authority.ready.filtered { "listen_filter" } else { "listen" }, port]).map_err(internal)?;
            db.kv_set(&endpoint_owner(participant, authority.ready.filtered), None).map_err(internal)?;
        }
        let owns = db.kv_get(&owner(participant)).map_err(internal)?.as_deref() == Some(id);
        if owns {
            if let (Some(row), Some(reason)) = (row, reason) {
                let context = match reason {
                    ListenEnd::Message => "message received",
                    ListenEnd::Match => "filter matched",
                    ListenEnd::Timeout => "listen timeout",
                    ListenEnd::DeliveryFailed => "delivery failed",
                };
                status(db, &row, "inactive", context)?;
            }
            db.kv_set(&owner(participant), None).map_err(internal)?;
        }
        authority.ended = true;
        save(db, context, &authority)
    })
}

#[cfg(test)]
#[path = "listening_tests.rs"]
mod tests;
