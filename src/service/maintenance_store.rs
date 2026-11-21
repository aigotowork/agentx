//! Daemon-owned archive and maintenance adapter. Never invoked by a client.
use super::maintenance::*;
use crate::db::{AgentxDb, DEV_ROOT_KV_KEY};
use crate::paths::ARCHIVE_DIR;
use crate::service::{RequestContext, ServiceError, ServiceResult};
use std::path::{Path, PathBuf};

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}
fn open_archive(path: &Path) -> rusqlite::Result<rusqlite::Connection> {
    rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
}

pub(crate) fn list_archives(base: &Path, cwd: Option<&str>, here_filter: bool) -> Vec<serde_json::Value> {
    let archive_dir = base.join(ARCHIVE_DIR);
    if !archive_dir.exists() {
        return Vec::new();
    }

    let mut session_dirs: Vec<PathBuf> =
        std::fs::read_dir(&archive_dir).ok().map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir() && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("session-"))).collect()).unwrap_or_default();

    // Sort newest first
    session_dirs.sort_by(|a, b| b.cmp(a));

    let mut archives = Vec::new();

    for session_dir in &session_dirs {
        let db_path = session_dir.join("agentx.db");
        if !db_path.exists() {
            continue;
        }

        let dir_name = session_dir.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();

        let timestamp = dir_name.strip_prefix("session-").unwrap_or("").to_string();

        let (event_count, instance_count) = match query_archive_counts(&db_path, cwd, here_filter) {
            Some((e, i)) => (Some(e), Some(i)),
            None => continue, // filtered out by --here, or error
        };

        // Get file metadata
        let (size_bytes, created) = match std::fs::metadata(&db_path) {
            Ok(meta) => {
                let size = meta.len() as i64;
                let mtime = meta.modified().map(crate::shared::system_time_to_epoch_f64).unwrap_or(0.0);
                (size, mtime)
            }
            Err(_) => (0, 0.0),
        };

        archives.push(serde_json::json!({
            "index": archives.len() + 1,
            "name": dir_name,
            "path": session_dir.to_string_lossy(),
            "timestamp": timestamp,
            "size_bytes": size_bytes,
            "created": created,
            "events": event_count,
            "instances": instance_count,
        }));
    }

    // Renumber after filtering
    for (i, a) in archives.iter_mut().enumerate() {
        a["index"] = serde_json::json!(i + 1);
    }

    archives
}

/// Query event/instance counts from an archive DB. Returns None if filtered out.
fn query_archive_counts(db_path: &Path, cwd: Option<&str>, here_filter: bool) -> Option<(i64, i64)> {
    let conn = open_archive(db_path).ok()?;
    let event_count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0)).unwrap_or(0);
    let instance_count: i64 = conn.query_row("SELECT COUNT(*) FROM instances", [], |r| r.get(0)).unwrap_or(0);

    if here_filter && let Some(cwd) = cwd {
        let has_match: bool = conn.query_row("SELECT 1 FROM instances WHERE directory = ?1 LIMIT 1", rusqlite::params![cwd], |_| Ok(true)).unwrap_or(false);
        if !has_match {
            return None;
        }
    }

    Some((event_count, instance_count))
}

/// Resolve archive by index (1-based) or name prefix.
fn resolve_archive<'a>(selector: &str, archives: &'a [serde_json::Value]) -> Option<&'a serde_json::Value> {
    // Try as index
    if let Ok(idx) = selector.parse::<usize>()
        && idx >= 1
        && idx <= archives.len()
    {
        return Some(&archives[idx - 1]);
    }

    // Try as name or prefix match
    for archive in archives {
        let name = archive["name"].as_str().unwrap_or("");
        if name == selector || name.contains(selector) {
            return Some(archive);
        }
    }

    None
}

/// Query events from an archive database.
fn query_archive_events(archive: &serde_json::Value, sql_filter: Option<&str>, last: usize) -> Result<Vec<serde_json::Value>, String> {
    let path = archive["path"].as_str().ok_or("Invalid archive path")?;
    let db_path = PathBuf::from(path).join("agentx.db");
    let conn = open_archive(&db_path).map_err(|e| e.to_string())?;

    let query = if let Some(filter) = sql_filter {
        // Try events_v view first
        let has_view = conn.query_row("SELECT 1 FROM events_v LIMIT 1", [], |_| Ok(true)).is_ok();
        if has_view {
            format!("SELECT id, timestamp, type, instance, data FROM events_v WHERE {filter} ORDER BY id DESC LIMIT {last}")
        } else {
            format!("SELECT id, timestamp, type, instance, data FROM events WHERE {filter} ORDER BY id DESC LIMIT {last}")
        }
    } else {
        format!("SELECT id, timestamp, type, instance, data FROM events ORDER BY id DESC LIMIT {last}")
    };

    crate::events::predicate::read(&conn, crate::events::predicate::Projection::Events, || {
        let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                let timestamp: String = row.get(1)?;
                let etype: String = row.get(2)?;
                let instance: String = row.get(3)?;
                let data_str: String = row.get::<_, String>(4).unwrap_or_default();
                let data: serde_json::Value = serde_json::from_str(&data_str).unwrap_or(serde_json::json!({}));
                Ok(serde_json::json!({
                    "id": id,
                    "timestamp": timestamp,
                    "type": etype,
                    "instance": instance,
                    "data": data,
                }))
            })
            .map_err(|e| e.to_string())?;

        let mut events: Vec<serde_json::Value> = rows.filter_map(|r| r.ok()).collect();
        events.reverse(); // Show oldest first
        Ok(events)
    })
}

/// Query instances from an archive database.
fn query_archive_instances(archive: &serde_json::Value, sql_filter: Option<&str>) -> Result<Vec<serde_json::Value>, String> {
    let path = archive["path"].as_str().ok_or("Invalid archive path")?;
    let db_path = PathBuf::from(path).join("agentx.db");
    let conn = open_archive(&db_path).map_err(|e| e.to_string())?;

    let query = if let Some(filter) = sql_filter {
        format!("SELECT name, status, directory, transcript_path, session_id FROM instances WHERE {filter} ORDER BY created_at DESC")
    } else {
        "SELECT name, status, directory, transcript_path, session_id FROM instances ORDER BY created_at DESC".to_string()
    };

    crate::events::predicate::read(&conn, crate::events::predicate::Projection::Participants, || {
        let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "name": row.get::<_, String>(0).unwrap_or_default(),
                    "status": row.get::<_, String>(1).unwrap_or_default(),
                    "directory": row.get::<_, String>(2).unwrap_or_default(),
                    "transcript_path": row.get::<_, String>(3).unwrap_or_default(),
                    "session_id": row.get::<_, String>(4).unwrap_or_default(),
                }))
            })
            .map_err(|e| e.to_string())?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    })
}

/// Snapshot the live WAL database before the clear transaction. VACUUM INTO
/// produces one consistent standalone archive and never copies live sidecars.
fn reset_conversation(db: &AgentxDb) -> ServiceResult<ResetResult> {
    let base = db.path().parent().unwrap_or(Path::new("."));
    let has_content: bool = db.conn().query_row("SELECT EXISTS(SELECT 1 FROM events) OR EXISTS(SELECT 1 FROM instances)", [], |row| row.get(0)).map_err(internal)?;
    let archive = if has_content {
        let stamp = chrono::Local::now().format("%Y-%m-%d_%H%M%S");
        let target = base.join(ARCHIVE_DIR).join(format!("session-{stamp}-{}", uuid::Uuid::new_v4()));
        crate::paths::ensure_private_directory(&target).map_err(internal)?;
        let destination = target.join("agentx.db");
        db.conn().execute("VACUUM INTO ?1", rusqlite::params![destination.to_string_lossy().as_ref()]).map_err(internal)?;
        crate::paths::ensure_private_db(&destination).map_err(internal)?;
        Some(target.to_string_lossy().into_owned())
    } else {
        None
    };
    let generation = uuid::Uuid::new_v4().to_string();
    db.with_write_scope(|| {
        db.clear_managed_state_for_reset()?;
        db.kv_set("service:database_generation", Some(&generation))?;
        db.log_reset_event()?;
        Ok(())
    })
    .map_err(internal)?;
    Ok(ResetResult { archive, database_generation: generation })
}

pub(crate) fn handle(db: &AgentxDb, context: &RequestContext, request: Request) -> ServiceResult<Response> {
    let base = db.path().parent().unwrap_or(Path::new("."));
    let cwd = context.caller.invocation.cwd.to_string_lossy();
    match request {
        Request::ReapDeadProcesses => crate::agents::service::cleanup_system(db, context, crate::agents::service::SystemCleanup::DeadProcesses).map(Response::Cleanup),
        Request::Summary => {
            let mut statement = db.conn().prepare("SELECT name FROM instances WHERE origin_device_id IS NULL OR origin_device_id = '' ORDER BY created_at DESC").map_err(internal)?;
            let local_participants = statement.query_map([], |row| row.get::<_, String>(0)).map_err(internal)?.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)?;
            let event_count = db.conn().query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0)).map_err(internal)?;
            Ok(Response::Summary(DatabaseSummary { local_participants, event_count, database_generation: context.database_generation.clone(), database_path: db.path().to_path_buf() }))
        }
        Request::DevRoot => {
            let value = db.kv_get(DEV_ROOT_KV_KEY);
            crate::service::context::persistent_dev_root(value.as_ref().map(|value| value.as_deref().filter(|value| !value.is_empty())).map_err(|_| ()));
            Ok(Response::DevRoot(value.map_err(internal)?.filter(|value| !value.is_empty())))
        }
        Request::SetDevRoot { value } => {
            db.kv_set(DEV_ROOT_KV_KEY, value.as_deref()).map_err(internal)?;
            Ok(Response::Updated)
        }
        Request::TipSeen { participant, tip } => Ok(Response::TipSeen(participant.is_empty() || db.kv_get(&format!("tip:{participant}:{tip}")).map_err(internal)?.is_some())),
        Request::MarkTipSeen { participant, tip } => {
            if !participant.is_empty() {
                db.kv_set(&format!("tip:{participant}:{tip}"), Some("1")).map_err(internal)?;
            }
            Ok(Response::Updated)
        }
        Request::Archives { here } => Ok(Response::Archives(list_archives(base, Some(&cwd), here))),
        Request::ArchivedEvents { selector, here, predicate, limit } => {
            let archives = list_archives(base, Some(&cwd), here);
            let archive = resolve_archive(&selector, &archives).ok_or_else(|| ServiceError::NotFound(format!("Archive '{selector}' not found")))?;
            query_archive_events(archive, predicate.as_deref(), limit).map(Response::ArchivedEvents).map_err(ServiceError::InvalidInput)
        }
        Request::ArchivedParticipants { selector, here, predicate } => {
            let archives = list_archives(base, Some(&cwd), here);
            let archive = resolve_archive(&selector, &archives).ok_or_else(|| ServiceError::NotFound(format!("Archive '{selector}' not found")))?;
            query_archive_instances(archive, predicate.as_deref()).map(Response::ArchivedParticipants).map_err(ServiceError::InvalidInput)
        }
        Request::ResetConversation => crate::service::context::without_dataset_observation(|| reset_conversation(db)).map(Response::Reset),
    }
}
