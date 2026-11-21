//! Daemon-owned agent listing projection; CLI owns its display and local transcript usage.
use super::super::queries::{Bindings, Listing, StoppedEntry};
use crate::agents::lifecycle::RECENTLY_STOPPED_WINDOW;
use crate::agents::records::is_remote_instance;
use crate::agents::{identity, lifecycle};
use crate::db::{AgentxDb, InstanceRow};
use crate::service::{RequestContext, ServiceError, ServiceResult};
use std::collections::{HashMap, HashSet};
fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

pub(crate) fn names(db: &AgentxDb, filter: super::super::queries::AgentFilter) -> ServiceResult<Vec<String>> {
    if filter.labels.is_empty() {
        let mut statement = db.conn().prepare_cached("SELECT name,launch_context FROM instances ORDER BY created_at DESC").map_err(internal)?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))).map_err(internal)?;
        let mut names = Vec::new();
        for row in rows {
            let (name, context) = row.map_err(internal)?;
            if filter.matches_workspace(context.as_deref()) {
                names.push(name);
            }
        }
        return Ok(names);
    }
    let selectors = crate::agents::labels::normalize_selectors(&filter.labels)?;
    let query = crate::agents::labels::LabelQuery::visible_agents(&selectors, filter.workspace.as_selector(), None);
    let mut names = Vec::new();
    loop {
        let mut page = db.select_labeled_names(&query, names.last().map(String::as_str).unwrap_or_default()).map_err(internal)?;
        let more = page.len() > query.limit as usize;
        page.truncate(query.limit as usize);
        names.extend(page);
        if !more {
            return Ok(names);
        }
    }
}

pub(crate) fn load(db: &AgentxDb, context: &RequestContext, target: Option<&str>) -> ServiceResult<Listing> {
    db.with_write_scope(|| load_in_scope(db, context, target).map_err(anyhow::Error::new)).map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

fn load_in_scope(db: &AgentxDb, context: &RequestContext, target: Option<&str>) -> ServiceResult<Listing> {
    let cleanup = crate::agents::service::cleanup_system(db, context, crate::agents::service::SystemCleanup::Listing)?;
    for record in db.iter_instances_full().map_err(internal)? {
        if !is_remote_instance(&record) && crate::agents::records::is_launching_placeholder(&record) {
            lifecycle::try_finalize_launch_failure_detail_stored(db, &record, None)?;
        }
    }
    let records = db.iter_instances_full().map_err(internal)?;
    project(db, context, target, records, None, cleanup)
}
pub(crate) fn load_labels(db: &AgentxDb, context: &RequestContext, target: Option<&str>, selectors: Vec<String>, workspace: Option<String>) -> ServiceResult<Listing> {
    let selectors = selectors.iter().map(|text| crate::agents::labels::parse_selector(text)).collect::<ServiceResult<Vec<_>>>()?;
    db.with_write_scope(|| {
        let cleanup = crate::agents::service::cleanup_system(db, context, crate::agents::service::SystemCleanup::Listing)?;
        let (mut records, snapshots) = select_labeled(db, context, &selectors, workspace.as_deref())?;
        if let Some(name) = target {
            records.retain(|row| row.name == name);
        }
        project(db, context, target, records, Some(snapshots), cleanup).map_err(anyhow::Error::new)
    })
    .map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

/// Shared read-only native label selection; listing cleanup is not part of this operation.
pub(super) fn select_labeled(
    db: &AgentxDb,
    context: &RequestContext,
    selectors: &[crate::agents::labels::LabelPredicate],
    workspace: Option<&str>,
) -> ServiceResult<(Vec<InstanceRow>, std::collections::BTreeMap<String, crate::agents::labels::LabelSnapshot>)> {
    let mut records = Vec::new();
    let mut snapshots = std::collections::BTreeMap::new();
    let mut after = None;
    loop {
        let query = crate::agents::labels::LabelQuery::visible_agents(selectors, workspace, after);
        let page = crate::agents::labels::select_current(db, context, query)?;
        records.extend(page.records);
        snapshots.extend(page.snapshots);
        after = page.next;
        if after.is_none() {
            return Ok((records, snapshots));
        }
    }
}
fn project(
    db: &AgentxDb,
    context: &RequestContext,
    target: Option<&str>,
    records: Vec<InstanceRow>,
    snapshots: Option<std::collections::BTreeMap<String, crate::agents::labels::LabelSnapshot>>,
    cleanup: crate::agents::service::CleanupOutcome,
) -> ServiceResult<Listing> {
    let labels = if let Some(snapshots) = snapshots {
        snapshots
    } else {
        let mut labels = std::collections::BTreeMap::new();
        let participants: Vec<_> = records.iter().filter(|row| !is_remote_instance(row)).map(|row| crate::agents::attachments::ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() }).collect();
        for batch in participants.chunks(100) {
            labels.extend(crate::agents::labels::hydrate(db, context, batch)?);
        }
        labels
    };
    let selected_name = target.map(str::to_owned);
    let missing_target = target.filter(|_| !records.iter().any(|row| Some(&row.name) == selected_name.as_ref())).map(|name| identity::describe_missing_agent(db, name));
    let unread = get_unread_counts_batch(db, &records);
    let bindings = get_bindings_batch(db);
    let statuses = records.iter().map(|record| (record.name.clone(), lifecycle::get_instance_status_stored(record, db))).collect();
    let plugin_ports = records.iter().filter_map(|record| db.get_notify_endpoint_port(&record.name, "plugin").map(|port| (record.name.clone(), i64::from(port)))).collect();
    let active: HashSet<_> = records.iter().map(|record| record.name.clone()).collect();
    let recently_stopped = get_recently_stopped(db, &active);
    Ok(Listing { labels, cleanup, records, unread, bindings, statuses, plugin_ports, recently_stopped, selected_name, missing_target })
}

pub(crate) fn stopped_history(db: &AgentxDb, target: Option<&str>, limit: usize) -> ServiceResult<Vec<StoppedEntry>> {
    let target = target.map(str::to_owned);
    let mut statement = db.conn().prepare("SELECT instance,timestamp,data FROM events WHERE type = 'life' AND json_extract(data,'$.action') = 'stopped' AND (?1 IS NULL OR instance = ?1) ORDER BY id DESC LIMIT ?2").map_err(internal)?;
    let mut entries = statement
        .query_map(rusqlite::params![target, limit.min(10000) as i64], |row| Ok(StoppedEntry { instance: row.get(0)?, timestamp: row.get(1)?, data: row.get(2)? }))
        .map_err(internal)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(internal)?;
    for entry in &mut entries {
        let mut data: serde_json::Value = serde_json::from_str(&entry.data).map_err(internal)?;
        AgentxDb::project_event_data("life", &mut data, &[]);
        entry.data = serde_json::to_string(&data).map_err(internal)?;
    }
    Ok(entries)
}
pub(crate) fn get_bindings_batch(db: &AgentxDb) -> HashMap<String, Bindings> {
    let Ok(mut stmt) = db.conn().prepare(
        "SELECT name,
            EXISTS(SELECT 1 FROM session_bindings WHERE instance_name = instances.name),
            EXISTS(SELECT 1 FROM process_bindings WHERE instance_name = instances.name)
         FROM instances",
    ) else {
        return HashMap::new();
    };
    stmt.query_map([], |row| Ok((row.get(0)?, Bindings { session: row.get(1)?, process: row.get(2)? }))).ok().into_iter().flatten().filter_map(Result::ok).collect()
}

#[cfg(test)]
pub(crate) fn get_unread_count(db: &AgentxDb, name: &str, last_event_id: i64) -> i64 {
    db.conn()
        .query_row(
            "SELECT COUNT(*) FROM events WHERE id > ? AND type = 'message'
             AND EXISTS (SELECT 1 FROM json_each(json_extract(data, '$.delivered_to')) WHERE value = ?)",
            rusqlite::params![last_event_id, name],
            |row| row.get(0),
        )
        .unwrap_or(0)
}

/// Get unread counts for all instances in batch.
pub(crate) fn get_unread_counts_batch(db: &AgentxDb, instances: &[InstanceRow]) -> HashMap<String, i64> {
    try_get_unread_counts_batch(db, instances).unwrap_or_default().into_iter().filter(|(_, count)| *count > 0).collect()
}

/// Fallible shared unread projection. A failed read must not become a known zero.
/// Use covered participant keys to exclude unrelated events. Until backfill
/// covers the requested unread tail, scan that tail once for the whole roster.
pub(crate) fn try_get_unread_counts_batch(db: &AgentxDb, instances: &[InstanceRow]) -> ServiceResult<HashMap<String, i64>> {
    let recipients: HashMap<&str, i64> = instances.iter().filter(|inst| !is_remote_instance(inst)).map(|inst| (inst.name.as_str(), inst.last_event_id)).collect();
    let Some(min_cursor) = recipients.values().min() else {
        return Ok(HashMap::new());
    };
    if db.conversation_routes_cover(*min_cursor, i64::MAX) {
        return indexed_unread_counts(db, &recipients);
    }
    let mut stmt = db.conn().prepare("SELECT id, json_extract(data, '$.delivered_to') FROM events WHERE type = 'message' AND id > ? ORDER BY id").map_err(internal)?;
    let rows = stmt.query_map([min_cursor], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))).map_err(internal)?;
    let mut counts: HashMap<String, i64> = recipients.keys().map(|name| ((*name).to_owned(), 0)).collect();
    for row in rows {
        let (id, data) = row.map_err(internal)?;
        let Some(data) = data else { continue };
        let data: serde_json::Value = serde_json::from_str(&data).map_err(internal)?;
        let Some(delivered_to) = data.as_array() else { continue };
        let mut seen = HashSet::new();
        for name in delivered_to.iter().filter_map(serde_json::Value::as_str) {
            if let Some(cursor) = recipients.get(name)
                && id > *cursor
                && seen.insert(name)
            {
                *counts.get_mut(name).expect("selected recipient initialized") += 1;
            }
        }
    }
    Ok(counts)
}

fn indexed_unread_counts(db: &AgentxDb, recipients: &HashMap<&str, i64>) -> ServiceResult<HashMap<String, i64>> {
    // A conversation key is a candidate: outgoing events also have keys.
    // The original delivered_to array remains authoritative for unread counts.
    const SQL: &str = "SELECT cursor.key,json_extract(e.data,'$.delivered_to') FROM json_each(?1) cursor
        CROSS JOIN conversation_routes r ON r.participant=cursor.key AND r.event_id>cursor.value
        CROSS JOIN events e ON e.id=r.event_id WHERE e.type='message'";
    let cursors = serde_json::to_string(recipients).map_err(internal)?;
    let mut stmt = db.conn().prepare_cached(SQL).map_err(internal)?;
    let rows = stmt.query_map([cursors], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))).map_err(internal)?;
    let mut counts: HashMap<String, i64> = recipients.keys().map(|name| ((*name).to_owned(), 0)).collect();
    for row in rows {
        let (name, data) = row.map_err(internal)?;
        let Some(data) = data else { continue };
        let delivered: serde_json::Value = serde_json::from_str(&data).map_err(internal)?;
        if delivered.as_array().is_some_and(|values| values.iter().any(|value| value.as_str() == Some(name.as_str()))) {
            *counts.get_mut(&name).expect("selected recipient initialized") += 1;
        }
    }
    Ok(counts)
}

pub(crate) fn get_recently_stopped(db: &AgentxDb, exclude_active: &std::collections::HashSet<String>) -> Vec<String> {
    let now = crate::shared::time::now_epoch_f64();
    let cutoff = now - RECENTLY_STOPPED_WINDOW;
    let cutoff_ts = chrono::DateTime::from_timestamp(cutoff as i64, 0).map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string()).unwrap_or_default();

    let Ok(mut stmt) = db.conn().prepare(
        "SELECT DISTINCT instance FROM events
         WHERE type = 'life' AND json_extract(data, '$.action') = 'stopped'
         AND timestamp > ?
         ORDER BY id DESC",
    ) else {
        return vec![];
    };

    stmt.query_map(rusqlite::params![cutoff_ts], |row| row.get::<_, String>(0)).ok().into_iter().flatten().filter_map(|r| r.ok()).filter(|name| !exclude_active.contains(name)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stopped_history_projects_snapshot_and_keeps_event_identity() {
        let directory = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&directory.path().join("test.db")).unwrap();
        let data = json!({"action": "stopped", "reason": "finished", "snapshot": {"name": "luna", "tool": "codex", "retired_metadata": "old"}});
        db.conn().execute("INSERT INTO events(timestamp,type,instance,data) VALUES (?1,'life',?2,?3)", rusqlite::params!["2026-01-01T00:00:00Z", "luna", data.to_string()]).unwrap();

        let entries = stopped_history(&db, Some("luna"), 10).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].instance, "luna");
        assert_eq!(entries[0].timestamp, "2026-01-01T00:00:00Z");
        let projected: serde_json::Value = serde_json::from_str(&entries[0].data).unwrap();
        assert_eq!(projected["reason"], "finished");
        assert_eq!(projected["snapshot"]["tool"], "codex");
        assert!(projected["snapshot"].get("retired_metadata").is_none());
    }
}
