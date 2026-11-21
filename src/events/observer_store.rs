//! Exact temporary event observers. Persistence never waits for the caller;
//! the existing postcommit notification boundary invokes `wake` after commit.

use super::service::ObserverSpec;
use crate::db::AgentxDb;
use crate::service::{RequestContext, ServiceError, ServiceResult};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Authority {
    spec: ObserverSpec,
    peer: String,
    pid: u32,
    incarnation: String,
    ended: bool,
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn key(generation: &str, id: &str) -> String {
    format!("event_observer:{generation}:{id}")
}
fn endpoint(id: &str) -> String {
    format!("event-observer:{id}")
}
fn current(db: &AgentxDb, spec: &ObserverSpec) -> ServiceResult<bool> {
    let row = db.get_instance_full(&spec.participant.canonical_name).map_err(internal)?;
    if row.as_ref().is_some_and(|row| row.created_at.to_bits() != spec.participant.incarnation_bits) {
        return Err(ServiceError::StaleFence);
    }
    Ok(row.is_some())
}
fn authenticate(db: &AgentxDb, context: &RequestContext, spec: &ObserverSpec) -> ServiceResult<()> {
    if !current(db, spec)? || crate::agents::service::resolve_actor(db, context)?.participant != spec.participant {
        return Err(ServiceError::StaleFence);
    }
    Ok(())
}
fn save(db: &AgentxDb, generation: &str, authority: &Authority) -> ServiceResult<()> {
    db.kv_set(&key(generation, &authority.spec.id), Some(&serde_json::to_string(authority).map_err(internal)?)).map_err(internal)
}
fn load(db: &AgentxDb, context: &RequestContext, spec: &ObserverSpec) -> ServiceResult<Authority> {
    let raw = db.kv_get(&key(&context.database_generation, &spec.id)).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    let authority: Authority = serde_json::from_str(&raw).map_err(internal)?;
    if authority.spec != *spec || authority.peer != super::store::peer(context)? {
        return Err(ServiceError::StaleFence);
    }
    Ok(authority)
}
fn remove(db: &AgentxDb, generation: &str, authority: &mut Authority) -> ServiceResult<()> {
    db.conn().execute("DELETE FROM notify_endpoints WHERE instance=?1 AND kind='events_wait' AND port=?2", rusqlite::params![endpoint(&authority.spec.id), authority.spec.port]).map_err(internal)?;
    authority.ended = true;
    save(db, generation, authority)
}

pub(super) fn begin(db: &AgentxDb, context: &RequestContext, spec: ObserverSpec) -> ServiceResult<()> {
    let suffix = spec.id.strip_prefix("observe-").ok_or_else(|| ServiceError::InvalidInput("invalid event observer ID".into()))?;
    if spec.port == 0 || uuid::Uuid::parse_str(suffix).is_err() {
        return Err(ServiceError::InvalidInput("invalid event observer endpoint".into()));
    }
    let peer = super::store::peer(context)?;
    super::store::transaction(db, || {
        authenticate(db, context, &spec)?;
        if db.kv_get(&key(&context.database_generation, &spec.id)).map_err(internal)?.is_some() {
            let authority = load(db, context, &spec)?;
            if authority.ended {
                return Err(ServiceError::StaleFence);
            }
            return check(db, context, &spec);
        }
        db.upsert_notify_endpoint(&endpoint(&spec.id), "events_wait", spec.port).map_err(internal)?;
        save(db, &context.database_generation, &Authority { spec, peer, pid: context.process.pid, incarnation: context.process.incarnation.clone().unwrap(), ended: false })
    })
}

pub(super) fn check(db: &AgentxDb, context: &RequestContext, spec: &ObserverSpec) -> ServiceResult<()> {
    let authority = load(db, context, spec)?;
    authenticate(db, context, spec)?;
    let owns: bool = db.conn().query_row("SELECT EXISTS(SELECT 1 FROM notify_endpoints WHERE instance=?1 AND kind='events_wait' AND port=?2)", rusqlite::params![endpoint(&spec.id), spec.port], |row| row.get(0)).map_err(internal)?;
    if authority.ended || !owns {
        return Err(ServiceError::StaleFence);
    }
    Ok(())
}

pub(super) fn end(db: &AgentxDb, context: &RequestContext, spec: &ObserverSpec) -> ServiceResult<()> {
    super::store::transaction(db, || {
        let mut authority = load(db, context, spec)?;
        current(db, spec)?;
        if !authority.ended {
            remove(db, &context.database_generation, &mut authority)?;
        }
        Ok(())
    })
}

/// Called only after `defer_event` says the event is committed. Abruptly dead
/// observers are reaped with positive process/participant evidence. Unknown OS
/// identity never authorizes connecting to a potentially reused TCP endpoint.
pub(crate) fn wake(db: &AgentxDb) {
    crate::observability::post_commit::measure(crate::observability::post_commit::Phase::Observers, || wake_committed(db));
}

fn wake_committed(db: &AgentxDb) {
    use crate::runtime::notify::observer_hints::Admission;
    let Ok(Some(generation)) = db.kv_get("service:database_generation") else {
        return;
    };
    let rows = (|| -> rusqlite::Result<Vec<(String, u16)>> {
        let mut statement = db.conn().prepare("SELECT instance,port FROM notify_endpoints WHERE kind='events_wait' AND instance LIKE 'event-observer:observe-%'")?;
        statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect()
    })();
    let Ok(rows) = rows else {
        return;
    };
    let mut ports = Vec::new();
    let mut ordinary = Vec::new();
    for (name, port) in rows {
        let Some(id) = name.strip_prefix("event-observer:") else {
            continue;
        };
        let Ok(Some(raw)) = db.kv_get(&key(&generation, id)) else {
            continue;
        };
        let Ok(mut authority) = serde_json::from_str::<Authority>(&raw) else {
            continue;
        };
        if authority.ended || authority.spec.id != id || authority.spec.port != port {
            continue;
        }
        let observed = crate::sys::process::identity(authority.pid);
        let dead = observed.as_deref().is_some_and(|actual| actual != authority.incarnation) || crate::sys::process::has_exited(authority.pid);
        let participant_live = current(db, &authority.spec);
        if dead || matches!(participant_live, Ok(false) | Err(ServiceError::StaleFence)) {
            let removed = super::store::transaction(db, || remove(db, &generation, &mut authority));
            // A retired participant's still-live observing process must wake
            // to see its stale fence. Never notify a dead/reused process owner.
            if removed.is_ok() && !dead && observed.as_deref() == Some(authority.incarnation.as_str()) {
                ports.push(port);
            }
        } else if matches!(participant_live, Ok(true)) && observed.as_deref() == Some(authority.incarnation.as_str()) {
            if let (Some(queue), Some(registration)) = (db.observer_hints(), id.strip_prefix("observe-").and_then(|id| uuid::Uuid::parse_str(id).ok())) {
                ordinary.push((queue, registration, authority.pid, authority.incarnation, port));
            } else {
                ports.push(port);
            }
        }
    }
    // Queue only after any synchronous stale-owner cleanup above. Its writes
    // revoke earlier offers; all these registrations passed the same scan.
    for (queue, registration, pid, incarnation, port) in ordinary {
        let admission = queue.offer(registration, &generation, pid, &incarnation, port);
        crate::observability::post_commit::count(|work| {
            let counter = match admission { Admission::Queued => &mut work.observer_hints_queued, Admission::Coalesced => &mut work.observer_hints_coalesced, Admission::Dropped => &mut work.observer_hints_dropped };
            *counter = counter.saturating_add(1);
        });
    }
    crate::runtime::notify::wake_ports(&ports, crate::runtime::notify::wake::WAKE_FANOUT_MS);
}

#[cfg(test)]
#[path = "observer_tests.rs"]
mod tests;
