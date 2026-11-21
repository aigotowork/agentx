//! Daemon-owned transcript correlation. Stored references only: Provider files,
//! content detection and timestamp interpretation belong to the caller host.
use super::service::{CodexLookup, Selection, Source, TrackedSource, correlation_key};
use crate::agents::attachments::ParticipantRef;
use crate::db::AgentxDb;
use crate::service::{ServiceError, ServiceResult};
use rusqlite::OptionalExtension;
use std::collections::{HashMap, HashSet};

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

pub(crate) fn select(db: &AgentxDb, name: &str, expected: Option<&ParticipantRef>) -> ServiceResult<Selection> {
    if let Some(expected) = expected {
        if expected.canonical_name != name {
            return Err(ServiceError::StaleFence);
        }
        let row = db.get_instance_full(name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        if row.created_at.to_bits() != expected.incarnation_bits {
            return Err(ServiceError::StaleFence);
        }
        return Ok(live_selection(row));
    }
    if let Some(row) = db.get_instance_full(name).map_err(internal)? {
        return Ok(live_selection(row));
    }
    let stopped = db.conn().query_row("SELECT EXISTS(SELECT 1 FROM events WHERE type='life' AND instance=?1 AND json_extract(data,'$.action')='stopped')", [name], |row| row.get::<_, bool>(0)).map_err(internal)?;
    let known = if stopped {
        Some(name.to_owned())
    } else {
        let pattern = format!("{}:____", name.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
        db.conn().query_row("SELECT name FROM instances WHERE name LIKE ?1 ESCAPE '\\' ORDER BY status_time DESC LIMIT 1", [pattern], |row| row.get::<_, String>(0)).optional().map_err(internal)?
    };
    let source = if let Some(name) = &known {
        exact(db, name)?
    } else {
        let pattern = format!("{}%", name.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
        let mut statement = db.conn().prepare("SELECT name,transcript_path,tool,session_id FROM instances WHERE name LIKE ?1 ESCAPE '\\' AND transcript_path IS NOT NULL AND transcript_path!='' LIMIT 2").map_err(internal)?;
        let mut rows = statement.query_map([pattern], |row| Ok(Source { name: row.get(0)?, path: row.get(1)?, tool: row.get(2)?, session_id: row.get(3)? })).map_err(internal)?.collect::<rusqlite::Result<Vec<_>>>().map_err(internal)?;
        if rows.len() == 1 { rows.pop() } else { None }
    };
    let canonical_name = source.as_ref().map(|source| source.name.clone()).or(known.clone()).unwrap_or_else(|| name.into());
    let missing = if source.is_none() && known.is_none() { Some(crate::agents::identity::describe_missing_agent(db, name)) } else { None };
    Ok(Selection { canonical_name, source, missing, codex_lookup: None })
}

fn live_selection(row: crate::db::InstanceRow) -> Selection {
    let codex_lookup = if row.tool == "codex" && row.origin_device_id.is_none() {
        (|| {
            let data: serde_json::Value = serde_json::from_str(row.launch_context.as_deref()?).ok()?;
            let home = std::path::PathBuf::from(data.get("codex_home")?.as_str()?);
            let session = row.session_id.clone().filter(|session| !session.is_empty())?;
            home.is_absolute().then_some(CodexLookup { home, session })
        })()
    } else { None };
    Selection { canonical_name: row.name.clone(), source: (!row.transcript_path.is_empty()).then_some(Source { name: row.name, path: row.transcript_path, tool: row.tool, session_id: row.session_id }), missing: None, codex_lookup }
}
fn exact(db: &AgentxDb, name: &str) -> ServiceResult<Option<Source>> {
    if let Some(row) = db.get_instance_full(name).map_err(internal)? {
        return Ok((!row.transcript_path.is_empty()).then_some(Source { name: row.name, path: row.transcript_path, tool: row.tool, session_id: row.session_id }));
    }
    let stopped:Option<(Option<String>,Option<String>,Option<String>)>=db.conn().query_row("SELECT json_extract(data,'$.snapshot.transcript_path'),json_extract(data,'$.snapshot.tool'),json_extract(data,'$.snapshot.session_id') FROM events WHERE type='life' AND instance=? AND json_extract(data,'$.action')='stopped' ORDER BY id DESC LIMIT 1",[name],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(internal)?;
    let Some((Some(path), tool, session_id)) = stopped else {
        return Ok(None);
    };
    if path.is_empty() {
        return Ok(None);
    }
    let tool = tool.unwrap_or_else(|| "unknown".into());
    Ok(Some(Source { name: name.into(), path, tool, session_id }))
}

pub(crate) fn inventory(db: &AgentxDb, live_only: bool) -> ServiceResult<Vec<TrackedSource>> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut statement = db.conn().prepare("SELECT name,transcript_path,tool,session_id FROM instances WHERE transcript_path IS NOT NULL AND transcript_path!=''").map_err(internal)?;
    for row in statement.query_map([], |row| Ok(Source { name: row.get(0)?, path: row.get(1)?, tool: row.get(2)?, session_id: row.get(3)? })).map_err(internal)? {
        let source = row.map_err(internal)?;
        if seen.insert(source.name.clone()) {
            out.push(TrackedSource { source, stopped_at: None });
        }
    }
    if live_only {
        return Ok(out);
    }
    let mut statement=db.conn().prepare("SELECT instance,json_extract(data,'$.snapshot.transcript_path') AS tp,json_extract(data,'$.snapshot.tool'),json_extract(data,'$.snapshot.session_id'),timestamp FROM events WHERE type='life' AND json_extract(data,'$.action')='stopped' AND tp IS NOT NULL AND tp!='' ORDER BY id DESC").map_err(internal)?;
    for row in statement
        .query_map([], |row| {
            let path: String = row.get(1)?;
            let tool = row.get::<_, Option<String>>(2)?.unwrap_or_else(|| "unknown".into());
            Ok(TrackedSource { source: Source { name: row.get(0)?, path, tool, session_id: row.get(3)? }, stopped_at: Some(row.get(4)?) })
        })
        .map_err(internal)?
    {
        let source = row.map_err(internal)?;
        if seen.insert(source.source.name.clone()) {
            out.push(source);
        }
    }
    Ok(out)
}

pub(crate) fn correlate(db: &AgentxDb, targets: &[(String, Option<String>)]) -> ServiceResult<HashMap<String, String>> {
    let mut result = HashMap::new();
    let targets: HashSet<_> = targets.iter().map(|(path, session)| correlation_key(path, session.as_deref())).collect();
    let mut statement = db.conn().prepare("SELECT name,transcript_path,session_id FROM instances WHERE transcript_path IS NOT NULL").map_err(internal)?;
    for row in statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))).map_err(internal)? {
        let (name, path, session) = row.map_err(internal)?;
        let key = correlation_key(&path, session.as_deref());
        if targets.contains(&key) {
            result.insert(key, name);
        }
    }
    let mut statement = db
        .conn()
        .prepare("SELECT instance,json_extract(data,'$.snapshot.transcript_path') AS tp,json_extract(data,'$.snapshot.session_id') FROM events WHERE type='life' AND json_extract(data,'$.action')='stopped' AND tp IS NOT NULL ORDER BY id DESC")
        .map_err(internal)?;
    for row in statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))).map_err(internal)? {
        let (name, path, session) = row.map_err(internal)?;
        let key = correlation_key(&path, session.as_deref());
        if targets.contains(&key) {
            result.entry(key).or_insert(name);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn native_transcript_inventory_and_correlation_preserve_session_identity() {
        let directory = tempfile::Builder::new().prefix("agentx-transcript-native-").tempdir().unwrap();
        let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
        for (name, session) in [("luna", "session-a"), ("nova", "session-b")] {
            db.save_instance_named(name, json!({"tool":"opencode","created_at":100.0,"transcript_path":"/origin/opencode.db","session_id":session}).as_object().unwrap()).unwrap();
        }
        for (name, path, session) in [("luna", "/obsolete/luna.jsonl", "old-luna"), ("gone", "/origin/stopped.jsonl", "gone-a"), ("gone", "/origin/latest.jsonl", "gone-b")] {
            db.log_life_event(name, "stopped", "cli", "self", Some(json!({"tool":"claude","transcript_path":path,"session_id":session}))).unwrap();
        }
        let sources = inventory(&db, false).unwrap();
        assert_eq!(sources.len(), 3);
        assert!(sources[..2].iter().all(|source| source.stopped_at.is_none()));
        assert_eq!(sources[2].source.path, "/origin/latest.jsonl");
        assert_eq!(inventory(&db, true).unwrap().len(), 2);
        let names = correlate(&db, &[("/origin/opencode.db".into(), Some("session-a".into())), ("/origin/opencode.db".into(), Some("session-b".into())), ("/origin/latest.jsonl".into(), Some("gone-b".into()))]).unwrap();
        assert_eq!(names.get(&correlation_key("/origin/opencode.db", Some("session-a"))).map(String::as_str), Some("luna"));
        assert_eq!(names.get(&correlation_key("/origin/opencode.db", Some("session-b"))).map(String::as_str), Some("nova"));
        assert_eq!(names.get(&correlation_key("/origin/latest.jsonl", Some("gone-b"))).map(String::as_str), Some("gone"));
    }

    #[test]
    fn native_transcript_source_preserves_exact_empty_live_and_remote_route() {
        let directory = tempfile::Builder::new().prefix("agentx-transcript-selection-").tempdir().unwrap();
        let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
        db.log_life_event("luna", "stopped", "cli", "self", Some(json!({"tool":"claude","transcript_path":"/old/luna.jsonl"}))).unwrap();
        db.save_instance_named("luna", json!({"created_at":100.0,"tool":"adhoc"}).as_object().unwrap()).unwrap();
        assert!(select(&db, "luna", None).unwrap().source.is_none());
        db.save_instance_named("nova:ABCD", json!({"created_at":100.0,"tool":"qoder","origin_device_id":"remote","transcript_path":"/remote/nova.jsonl"}).as_object().unwrap()).unwrap();
        assert_eq!(select(&db, "nova", None).unwrap().canonical_name, "nova:ABCD");
        let wrong = ParticipantRef { canonical_name: "luna".into(), incarnation_bits: 101.0f64.to_bits() };
        assert_eq!(select(&db, "luna", Some(&wrong)).unwrap_err(), ServiceError::StaleFence);
    }

    #[test]
    fn native_transcript_read_failures_do_not_become_empty_results() {
        let directory = tempfile::Builder::new().prefix("agentx-transcript-failure-").tempdir().unwrap();
        let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
        db.conn().execute_batch("ALTER TABLE instances RENAME TO unavailable_instances").unwrap();
        assert!(inventory(&db, false).is_err());
        assert!(correlate(&db, &[]).is_err());
        assert!(select(&db, "luna", None).is_err());
    }
}
