//! Workspace catalogue and availability reads for both cloud bearer surfaces.
use super::types::{WorkspacePage, WorkspaceQuery, WorkspaceView};
use crate::cloud::{
    ScopeContext,
    communication::store::{self, Error, Result},
};
use rusqlite::{Connection, params};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{LazyLock, Mutex},
    time::Duration,
};

pub(super) fn page(conn: &Connection, now: i64, generation: String, query: &WorkspaceQuery) -> Result<WorkspacePage> {
    if let Some(host) = &query.host {
        store::host_name(host)?;
    }
    let (after_host, after_workspace) = match query.after.as_deref() {
        Some(value) => {
            let (host, id) = value.split_once('/').ok_or(Error::Invalid("invalid workspace cursor"))?;
            store::host_name(host)?;
            store::workspace_id(id)?;
            (host, id)
        }
        None => ("", ""),
    };
    let limit = query.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return Err(Error::Invalid("workspace page limit must be between 1 and 100"));
    }

    let mut stmt = conn.prepare(
        "SELECT l.host,l.expires_at,j.value
         FROM control_leases l JOIN json_each(l.workspaces) j
         WHERE l.is_current=1 AND (?1 IS NULL OR l.host=?1) AND l.host>=?2
           AND (l.host,json_extract(j.value,'$.workspace_id'))>(?2,?3)
         ORDER BY l.host,json_extract(j.value,'$.workspace_id') LIMIT ?4",
    )?;
    let mut rows = stmt.query(params![query.host, after_host, after_workspace, limit + 1])?;
    let mut items = Vec::new();
    let mut next = None;
    while let Some(row) = rows.next()? {
        if items.len() == limit as usize {
            next = items.last().map(|item: &WorkspaceView| format!("{}/{}", item.host, item.capability.workspace_id));
            break;
        }
        items.push(WorkspaceView { host: row.get(0)?, online: row.get::<_, i64>(1)? > now, capability: serde_json::from_str(&row.get::<_, String>(2)?)? });
    }
    Ok(WorkspacePage { generation, items, next })
}

// At most one timer per active Namespace, shared by both /changes surfaces.
// A new lease always expires after already-live leases, so it cannot shorten a running timer.
static WATCHERS: LazyLock<Mutex<HashMap<(PathBuf, String), u64>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub(in crate::cloud) fn observe_workspace_availability(scope: &ScopeContext) {
    let key = (scope.root.clone(), scope.generation.clone());
    let mut watchers = WATCHERS.lock().unwrap();
    if let Some(version) = watchers.get_mut(&key) {
        *version = version.wrapping_add(1);
        return;
    }
    watchers.insert(key.clone(), 0);
    drop(watchers);
    tokio::spawn(watch_expiry(scope.clone(), key, store::now()));
}

async fn watch_expiry(scope: ScopeContext, key: (PathBuf, String), mut since: i64) {
    loop {
        let version = WATCHERS.lock().unwrap()[&key];
        let root = scope.root.clone();
        let generation = scope.generation.clone();
        let snapshot = tokio::task::spawn_blocking(move || store::transaction(&root, &generation, false, || true, |conn, now| expiry_window(conn, since, now))).await;
        if let Ok(Ok((now, expired, next))) = snapshot {
            if expired {
                scope.space_changed.send_modify(|snapshot| snapshot.roster = snapshot.roster.wrapping_add(1));
            }
            since = now;
            if let Some(next) = next {
                // Wall-clock seconds are integer lease boundaries. A renewed lease moves
                // forward in storage; checking it again at the old deadline emits nothing.
                tokio::time::sleep(Duration::from_secs(next.saturating_sub(now) as u64)).await;
                continue;
            }
        }

        let mut watchers = WATCHERS.lock().unwrap();
        if watchers.get(&key) == Some(&version) {
            watchers.remove(&key);
            return;
        }
        // A request/renewal may have started or extended a lease during the final read.
        // Check again before giving up ownership of this timer.
    }
}

pub(super) fn expiry_window(conn: &Connection, since: i64, now: i64) -> Result<(i64, bool, Option<i64>)> {
    let expired = conn.query_row("SELECT EXISTS(SELECT 1 FROM control_leases WHERE is_current=1 AND expires_at>?1 AND expires_at<=?2)", params![since, now], |row| row.get(0))?;
    let next = conn.query_row("SELECT min(expires_at) FROM control_leases WHERE is_current=1 AND expires_at>?1", [now], |row| row.get(0))?;
    Ok((now, expired, next))
}
