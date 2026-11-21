//! Resume source selection and the atomic source-to-launch boundary. Provider
//! files/argument composition remain in the originating runtime host.
use super::{internal, kill_peer, kill_transaction, participant};
use crate::agents::{attachments::ParticipantRef, lifecycle_service::*};
use crate::db::{AgentxDb, InstanceRow};
use crate::service::{RequestContext, ServiceError, ServiceResult};
use rusqlite::OptionalExtension;
use serde_json::{Value, json};
use sha2::Digest;

fn snapshot(row: &InstanceRow) -> ServiceResult<ResumeSnapshot> {
    Ok(ResumeSnapshot {
        tool: row.tool.clone(),
        session_id: row.session_id.clone().unwrap_or_default(),
        launch_args: row.launch_args.clone().unwrap_or_default(),
        launch_profile: row.launch_profile.as_deref().map(serde_json::from_str).transpose().map_err(internal)?,
        collaboration: crate::agents::workspaces::collaboration(row),
        title: crate::agents::launch_title::from_context(row.launch_context.as_deref()),
        background: row.background != 0,
        last_event_id: row.last_event_id,
        directory: row.directory.clone(),
        background_log_file: row.background_log_file.clone(),
    })
}
fn stopped(db: &AgentxDb, name: &str, event: Option<i64>) -> ServiceResult<Option<(i64, ResumeSnapshot)>> {
    let data: Option<(i64, String)> = db
        .conn()
        .query_row("SELECT id,data FROM events WHERE type='life' AND instance=?1 AND json_extract(data,'$.action')='stopped' AND (?2 IS NULL OR id=?2) ORDER BY id DESC LIMIT 1", rusqlite::params![name, event], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()
        .map_err(internal)?;
    data.map(|(id, data)| {
        let data: Value = serde_json::from_str(&data).map_err(internal)?;
        let value = data.get("snapshot").filter(|value| value.is_object()).ok_or_else(|| ServiceError::InvalidInput(format!("Stopped snapshot #{id} for '{name}' is missing or unreadable")))?;
        let text = |field: &str| value.get(field).and_then(Value::as_str).unwrap_or("").to_string();
        Ok((
            id,
            ResumeSnapshot {
                tool: text("tool"),
                session_id: text("session_id"),
                launch_args: text("launch_args"),
                launch_profile: value.get("launch_profile").and_then(serde_json::Value::as_str).filter(|value| !value.is_empty()).map(serde_json::from_str).transpose().map_err(internal)?,
                collaboration: stopped_collaboration(value)?,
                title: crate::agents::launch_title::from_snapshot(value),
                background: value.get("background").and_then(Value::as_i64).unwrap_or(0) != 0,
                last_event_id: value.get("last_event_id").and_then(Value::as_i64).unwrap_or(0),
                directory: text("directory"),
                background_log_file: text("background_log_file"),
            },
        ))
    })
    .transpose()
}
// Stopped snapshots retain the original serialized InstanceRow launch context.
fn stopped_collaboration(value: &Value) -> ServiceResult<Option<crate::agents::workspaces::LaunchCollaboration>> {
    let Some(context) = value.get("launch_context").filter(|value| !value.is_null()) else { return Ok(None) };
    let context: Value = if let Some(text) = context.as_str() { serde_json::from_str(text).map_err(internal)? } else { context.clone() };
    context.get("collaboration").filter(|value| !value.is_null()).map(|value| serde_json::from_value(value.clone()).map_err(internal)).transpose()
}

fn digest(snapshot: &ResumeSnapshot) -> ServiceResult<String> {
    Ok(sha2::Sha256::digest(serde_json::to_vec(snapshot).map_err(internal)?).iter().map(|byte| format!("{byte:02x}")).collect())
}
fn session_unbound(db: &AgentxDb, session: &str) -> ServiceResult<()> {
    if let Some(bound) = db.get_session_binding(session).map_err(internal)?
        && db.get_instance_full(&bound).map_err(internal)?.is_some()
    {
        return Err(ServiceError::Conflict(format!("Session {session} is currently active as '{bound}' — run agentx kill {bound} first")));
    }
    Ok(())
}
fn ensure_not_running(db: &AgentxDb, name: &str) -> ServiceResult<()> {
    use crate::agents::service::{TrackedProcessState, tracked_process_state};
    let Some(row) = db.get_instance_full(name).map_err(internal)? else {
        return Ok(());
    };
    let running = match tracked_process_state(&row) {
        TrackedProcessState::Alive => true,
        TrackedProcessState::Gone => false,
        TrackedProcessState::Unverifiable => row.status != "inactive",
    };
    if running {
        return Err(ServiceError::Conflict(format!("'{name}' is still running.\n  Branch a copy instead: agentx f {name}\n  Or stop it first:     agentx kill {name}")));
    }
    Ok(())
}

pub(super) fn resolve(db: &AgentxDb, context: &RequestContext, target: &str, expected: Option<&ParticipantRef>, fork: bool) -> ServiceResult<ResumeResolution> {
    select_retained(db, context, target, expected, fork, false)
}
pub(super) fn preview(db: &AgentxDb, context: &RequestContext, target: &str, fork: bool) -> ServiceResult<ResumeResolution> {
    select_retained(db, context, target, None, fork, true)
}
fn select_retained(db: &AgentxDb, context: &RequestContext, target: &str, expected: Option<&ParticipantRef>, fork: bool, allow_remote_preview: bool) -> ServiceResult<ResumeResolution> {
    kill_peer(context)?;
    kill_transaction(db, || {
        let result = crate::agents::service_store::retained(db, context, &json!({"resolve_resume":target,"expected":expected,"fork":fork,"preview":allow_remote_preview}), || {
            let mut cleanup = crate::agents::service::CleanupOutcome::default();
            let selection = select(db, context, target, expected, fork, allow_remote_preview, &mut cleanup)?;
            Ok(ResumeResolution { selection, cleanup })
        })?;
        crate::agents::hook_store::validate_stop_outcome(db, &crate::agents::hook_service::SessionStopOutcome { stopped: result.cleanup.retired > 0, retained: false, effects: result.cleanup.effects.clone() })?;
        if let ResumeSelection::Tracked { fence, .. } | ResumeSelection::Disk { fence, .. } = &result.selection {
            validate_selection(db, fence, allow_remote_preview)?;
        }
        Ok(result)
    })
}

fn select(db: &AgentxDb, context: &RequestContext, target: &str, expected: Option<&ParticipantRef>, fork: bool, allow_remote_preview: bool, cleanup: &mut crate::agents::service::CleanupOutcome) -> ServiceResult<ResumeSelection> {
    let mut name = target.to_owned();
    if let Some(expected) = expected {
        let row = db.get_instance_full(&expected.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        if participant(&row) != *expected || row.name != name {
            return Err(ServiceError::StaleFence);
        }
    }
    let session = uuid::Uuid::parse_str(&name).is_ok() || (name.starts_with("ses_") && name.len() >= 8 && name[4..].chars().all(|c| c.is_ascii_alphanumeric()));
    let mut pin = None;
    if session {
        session_unbound(db, &name)?;
        match db.find_stopped_event_by_session_id(&name).map_err(internal)? {
            Some((owner, event)) => {
                name = owner;
                pin = Some(event);
            }
            None => return Ok(ResumeSelection::Disk { session_id: name.clone(), fence: ResumeFence { name: None, participant: None, stopped_event: None, latest_stopped_event: None, snapshot_digest: None, session_id: name, fork, child: None } }),
        }
    }
    let original = db.get_instance_full(&name).map_err(internal)?;
    let remote = original.as_ref().is_some_and(|row| row.origin_device_id.is_some());
    if remote && !allow_remote_preview {
        return Err(ServiceError::InvalidInput("remote resume must execute on the owning device".into()));
    }
    if !fork {
        if remote {
            if original.as_ref().is_some_and(|row| row.status != "inactive") {
                return Err(ServiceError::Conflict(format!("'{name}' is still running.")));
            }
        } else {
            if let Some(row) = &original {
                *cleanup = crate::agents::cleanup_store::cleanup_in_write_scope(db, context, &crate::agents::service::SystemCleanup::Resume { participant: participant(row) })?;
                if !cleanup.deferred.is_empty() {
                    return Err(ServiceError::Conflict(format!("'{name}' has a tracked descendant still running — stop it before resuming")));
                }
            }
            ensure_not_running(db, &name)?;
        }
    }
    let current = db.get_instance_full(&name).map_err(internal)?;
    let latest = stopped(db, &name, None)?;
    let (selected, event) = if fork
        && pin.is_none()
        && let Some(row) = &current
    {
        (snapshot(row)?, None)
    } else if let Some((event, snapshot)) = stopped(db, &name, pin)? {
        (snapshot, Some(event))
    } else {
        let known = crate::agents::identity::known_agent_names(db);
        return Ok(ResumeSelection::Missing {
            diagnostic: format!("No agent named '{name}' to resume (not a known agentx agent, session UUID, or thread name){}\n  Stopped agents: agentx list --stopped", crate::shared::suggest::did_you_mean(&name, known.iter().map(String::as_str))),
        });
    };
    let mut selected = selected;
    if pin.is_some()
        && let Some((_, latest)) = &latest
    {
        selected.last_event_id = selected.last_event_id.max(latest.last_event_id);
    }
    let fence = ResumeFence {
        name: Some(name.clone()),
        participant: current.as_ref().map(participant),
        stopped_event: event,
        latest_stopped_event: latest.as_ref().map(|(id, _)| *id),
        snapshot_digest: Some(digest(&selected)?),
        session_id: selected.session_id.clone(),
        fork,
        child: None,
    };
    let preview_child_name = if fork { Some(crate::agents::names::allocate_unreserved_name(db).map_err(internal)?) } else { None };
    Ok(ResumeSelection::Tracked { name, snapshot: selected, fence, preview_child_name })
}

fn validate(db: &AgentxDb, source: &ResumeFence) -> ServiceResult<ResumeSnapshot> {
    validate_selection(db, source, false)
}

fn validate_selection(db: &AgentxDb, source: &ResumeFence, allow_remote_preview: bool) -> ServiceResult<ResumeSnapshot> {
    let Some(name) = source.name.as_deref() else {
        session_unbound(db, &source.session_id)?;
        if db.find_stopped_event_by_session_id(&source.session_id).map_err(internal)?.is_some() {
            return Err(ServiceError::StaleFence);
        }
        return Ok(ResumeSnapshot {
            tool: String::new(),
            session_id: source.session_id.clone(),
            launch_args: String::new(),
            launch_profile: None,
            collaboration: None,
            title: None,
            background: false,
            last_event_id: 0,
            directory: String::new(),
            background_log_file: String::new(),
        });
    };
    let row = db.get_instance_full(name).map_err(internal)?;
    if !allow_remote_preview && row.as_ref().is_some_and(|row| row.origin_device_id.is_some()) {
        return Err(ServiceError::StaleFence);
    }
    if row.as_ref().map(participant) != source.participant {
        return Err(ServiceError::StaleFence);
    }
    if !source.fork {
        ensure_not_running(db, name)?;
    }
    let latest = stopped(db, name, None)?;
    if latest.as_ref().map(|(id, _)| *id) != source.latest_stopped_event {
        return Err(ServiceError::StaleFence);
    }
    let mut selected = if let Some(event) = source.stopped_event { stopped(db, name, Some(event))?.ok_or(ServiceError::StaleFence)?.1 } else { snapshot(row.as_ref().ok_or(ServiceError::StaleFence)?)? };
    if source.stopped_event.is_some()
        && let Some((_, latest)) = latest
    {
        selected.last_event_id = selected.last_event_id.max(latest.last_event_id);
    }
    if source.snapshot_digest.as_deref() != Some(digest(&selected)?.as_str()) || source.session_id != selected.session_id {
        return Err(ServiceError::StaleFence);
    }
    Ok(selected)
}

pub(super) fn reserve_fork(db: &AgentxDb, context: &RequestContext, source: ResumeFence) -> ServiceResult<ResumeFence> {
    kill_peer(context)?;
    if !source.fork || source.name.is_none() {
        return Err(ServiceError::InvalidInput("tracked fork source required".into()));
    }
    kill_transaction(db, || {
        let result = crate::agents::service_store::retained(db, context, &json!({"resume_fork":source}), || {
            validate(db, &source)?;
            let config = crate::config::AgentxConfig::load_for_scope(db.path().parent().unwrap_or(std::path::Path::new(".")), &context.caller.invocation.environment);
            let name = crate::agents::names::reserve_generated_name_with_timeout(db, config.timeout).map_err(internal)?;
            let row = db.get_instance_full(&name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
            Ok(ResumeFence { child: Some(participant(&row)), ..source.clone() })
        })?;
        check_child(db, &result)?;
        Ok(result)
    })
}
fn check_child(db: &AgentxDb, source: &ResumeFence) -> ServiceResult<()> {
    let child = source.child.as_ref().ok_or(ServiceError::StaleFence)?;
    let row = db.get_instance_full(&child.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    if participant(&row) != *child || row.status != crate::agents::names::PLACEHOLDER_STATUS || row.status_context != crate::agents::names::PLACEHOLDER_CONTEXT || row.session_id.is_some() {
        return Err(ServiceError::StaleFence);
    }
    Ok(())
}

pub(super) fn reserve(db: &AgentxDb, context: &RequestContext, source: &ResumeFence, launch: crate::agents::launch_service::Reserve) -> ServiceResult<crate::agents::launch_service::Reservation> {
    kill_peer(context)?;
    kill_transaction(db, || {
        let result = crate::agents::service_store::retained(db, context, &json!({"resume":source,"launch":launch}), || {
            let selected = validate(db, source)?;
            if source.fork && source.name.is_some() {
                check_child(db, source)?;
            }
            let intended_name = if source.fork { source.child.as_ref().map(|child| &child.canonical_name) } else { source.name.as_ref() };
            if launch.name.as_ref() != intended_name || launch.prior_session != (!source.fork).then(|| source.session_id.clone()) || launch.resume_cursor != (!source.fork && source.name.is_some()).then_some(selected.last_event_id) {
                return Err(ServiceError::StaleFence);
            }
            let saved_tool = selected.tool.strip_suffix("-pty").unwrap_or(&selected.tool).parse::<crate::integrations::tool::Tool>().ok();
            if !selected.tool.is_empty() && selected.tool != "adhoc" && saved_tool != launch.tool.parse::<crate::integrations::tool::Tool>().ok() {
                return Err(ServiceError::StaleFence);
            }
            // Freeze exact old source before reservation replaces the row.
            let original = if !source.fork {
                if let Some(participant) = &source.participant {
                    Some(participant.clone())
                } else if let (Some(name), Some(event)) = (&source.name, source.stopped_event) {
                    let raw: String = db.conn().query_row("SELECT data FROM events WHERE id=?1", [event], |row| row.get(0)).map_err(internal)?;
                    let data: Value = serde_json::from_str(&raw).map_err(internal)?;
                    data.pointer("/snapshot/created_at").and_then(Value::as_f64).map(|created| ParticipantRef { canonical_name: name.clone(), incarnation_bits: created.to_bits() })
                } else {
                    None
                }
            } else {
                None
            };
            use crate::agents::launch_relations::{Creation, ForkSource};
            let creation = if source.fork {
                let fork_source = match (&source.name, source.stopped_event) {
                    (Some(name), Some(event)) => Some(ForkSource::Stopped { name: name.clone(), event }),
                    _ => source.participant.clone().map(ForkSource::Current),
                };
                Creation::Fork(fork_source)
            } else {
                Creation::Resume(original)
            };
            match crate::agents::launch_store::reserve_with_creation(db, context, launch.clone(), creation)? {
                crate::agents::launch_service::Response::Reserved(reservation) => Ok(reservation),
                _ => Err(ServiceError::Internal("unexpected native resume reservation response".into())),
            }
        })?;
        let row = db.get_instance_full(&result.participant.canonical_name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        if participant(&row) != result.participant || db.get_process_binding(&result.process_binding).map_err(internal)?.as_deref() != Some(row.name.as_str()) {
            return Err(ServiceError::StaleFence);
        }
        Ok(result)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stopped_history_preserves_adopted_role_prompt() {
        let adopted = json!({"workspace":"/original", "endpoint":true, "role":{"name":"reviewer","path":"/roles/reviewer.toml","namespace":"global:/roles","profile":"deep","prompt":"Original frozen role prompt","description":null,"default":true}});
        let context = json!({"collaboration":adopted});
        let stored = json!({"launch_context":context.to_string()});
        let restored = stopped_collaboration(&stored).unwrap().unwrap();
        assert_eq!(restored.workspace.as_deref(), Some("/original"));
        assert_eq!(restored.role.unwrap().prompt, "Original frozen role prompt");
        assert!(restored.endpoint);
        assert!(stopped_collaboration(&json!({})).unwrap().is_none());
    }

    fn context() -> RequestContext {
        RequestContext {
            caller: crate::service::CallerContext {
                request_id: crate::service::RequestId(uuid::Uuid::new_v4().to_string()),
                invocation: crate::service::InvocationContext { cwd: "/fixture".into(), ..Default::default() },
                deadline_unix_ms: None,
                expected_database_generation: Some("fixture".into()),
            },
            process: crate::service::CallerProcess { pid: 42, incarnation: Some("operator".into()) },
            daemon_instance: "fixture".into(),
            database_generation: "fixture".into(),
        }
    }
    fn stop(db: &AgentxDb, session: &str) -> i64 {
        db.log_event("life", "saved", &json!({"action":"stopped","snapshot":{"name":"saved","tool":"codex","session_id":session,"directory":"/fixture","last_event_id":0,"launch_args":"[]"}})).unwrap();
        db.get_last_event_id()
    }
    fn input(source: &ResumeFence) -> crate::agents::launch_service::Reserve {
        crate::agents::launch_service::Reserve {
            name: source.name.clone(),
            tool: "codex".into(),
            directory: "/fixture".into(),
            process_binding: "resume-process".into(),
            prior_session: Some(source.session_id.clone()),
            resume_cursor: Some(0),
            background: false,
            launch_args: vec![],
            launch_profile: None,
            collaboration: None,
            title: None,
        labels: Default::default(),
        }
    }
    #[test]
    fn resume_pins_conversation_but_uses_latest_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let older = uuid::Uuid::new_v4().to_string();
        let newer = uuid::Uuid::new_v4().to_string();
        let first = stop(&db, &older);
        let last = stop(&db, &newer);
        db.conn().execute("UPDATE events SET data=json_set(data,'$.snapshot.last_event_id',1) WHERE id=?", [last]).unwrap();
        let ResumeSelection::Tracked { snapshot, fence, .. } = resolve(&db, &context(), &older, None, false).unwrap().selection else { panic!("expected pinned source") };
        assert_eq!(snapshot.session_id, older);
        assert_eq!(snapshot.last_event_id, 1);
        assert_eq!(fence.stopped_event, Some(first));
        assert_eq!(fence.latest_stopped_event, Some(last));
        let context = context();
        let mut launch = input(&fence);
        launch.resume_cursor = Some(1);
        let first = reserve(&db, &context, &fence, launch.clone()).unwrap();
        let replay = reserve(&db, &context, &fence, launch).unwrap();
        assert_eq!(first, replay, "lost reservation reply cannot allocate twice");
        assert_eq!(db.iter_instances_full().unwrap().len(), 1);
    }
    #[test]
    fn resume_source_and_reservation_reject_intervening_namesake() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        stop(&db, &uuid::Uuid::new_v4().to_string());
        let ResumeSelection::Tracked { fence, .. } = resolve(&db, &context(), "saved", None, false).unwrap().selection else { panic!("expected stopped source") };
        db.conn().execute("INSERT INTO instances(name,tool,created_at,status) VALUES('saved','codex',5,'listening')", []).unwrap();
        assert_eq!(reserve(&db, &context(), &fence, input(&fence)).unwrap_err(), ServiceError::StaleFence);
        assert_eq!(db.get_instance_full("saved").unwrap().unwrap().created_at, 5.0);
    }
    #[test]
    fn resume_fork_reservation_uses_explicit_timeout_without_name_lock_file() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        db.conn().execute("INSERT INTO instances(name,tool,created_at,session_id,status) VALUES('saved','codex',1,'session','listening')", []).unwrap();
        let expected = ParticipantRef { canonical_name: "saved".into(), incarnation_bits: 1f64.to_bits() };
        let ResumeSelection::Tracked { fence, .. } = resolve(&db, &context(), "saved", Some(&expected), true).unwrap().selection else { panic!("expected live fork source") };
        let mut context = context();
        context.caller.invocation.environment.insert("AGENTX_TIMEOUT".into(), "321".into());
        let source = reserve_fork(&db, &context, fence).unwrap();
        let child = source.child.unwrap();
        assert_eq!(db.get_instance_full(&child.canonical_name).unwrap().unwrap().wait_timeout, Some(321));
        assert!(!dir.path().join(".tmp/name_gen.lock").exists());
    }
    #[test]
    fn resume_disk_adoption_rechecks_new_live_session_binding() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let session = uuid::Uuid::new_v4().to_string();
        let ResumeSelection::Disk { fence, .. } = resolve(&db, &context(), &session, None, false).unwrap().selection else { panic!("expected external disk source") };
        db.conn().execute("INSERT INTO instances(name,tool,created_at) VALUES('owner','codex',1)", []).unwrap();
        db.set_session_binding(&session, "owner").unwrap();
        let mut launch = input(&fence);
        launch.name = None;
        launch.resume_cursor = None;
        assert!(matches!(reserve(&db, &context(), &fence, launch), Err(ServiceError::Conflict(_))));
        assert_eq!(db.iter_instances_full().unwrap().len(), 1);
    }

    #[cfg(unix)]
    fn exited_source(db: &AgentxDb) -> ParticipantRef {
        assert!(crate::sys::process::identity(std::process::id()).is_some());
        db.conn().execute("INSERT INTO instances(name,tool,directory,created_at,session_id,status,launch_args) VALUES('saved','codex','/fixture',1,'dead-source-session','listening','[]')", []).unwrap();
        db.update_instance_pid_with_identity("saved", std::process::id(), Some("previous-os-incarnation")).unwrap();
        participant(&db.get_instance_full("saved").unwrap().unwrap())
    }

    #[cfg(unix)]
    #[test]
    fn resume_cleanup_replays_frozen_selection_after_deletion_and_rejects_successor() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let expected = exited_source(&db);
        let context = context();
        let first = resolve(&db, &context, "saved", Some(&expected), false).unwrap();
        assert_eq!(first.cleanup.retired, 1);
        assert_eq!(first.cleanup.effects.len(), 1);
        assert!(db.get_instance_full("saved").unwrap().is_none());
        let replay = resolve(&db, &context, "saved", Some(&expected), false).unwrap();
        assert_eq!(serde_json::to_value(&first).unwrap(), serde_json::to_value(&replay).unwrap());
        db.conn().execute("INSERT INTO instances(name,tool,created_at) VALUES('saved','codex',2)", []).unwrap();
        assert_eq!(resolve(&db, &context, "saved", Some(&expected), false).unwrap_err(), ServiceError::StaleFence);
    }

    #[cfg(unix)]
    #[test]
    fn resume_cleanup_retention_failure_rolls_back_and_same_request_retries() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let expected = exited_source(&db);
        let context = context();
        let before = db.get_last_event_id();
        db.conn().execute_batch("CREATE TRIGGER fail_resume_retention BEFORE INSERT ON kv WHEN NEW.key LIKE 'participant_request:%' BEGIN SELECT RAISE(ABORT,'retention unavailable'); END;").unwrap();
        assert!(resolve(&db, &context, "saved", Some(&expected), false).is_err());
        assert_eq!(participant(&db.get_instance_full("saved").unwrap().unwrap()), expected);
        assert_eq!(db.get_last_event_id(), before);
        db.conn().execute_batch("DROP TRIGGER fail_resume_retention;").unwrap();
        let recovered = resolve(&db, &context, "saved", Some(&expected), false).unwrap();
        assert_eq!(recovered.cleanup.retired, 1);
        assert!(db.get_instance_full("saved").unwrap().is_none());
    }
}

#[cfg(test)]
mod title_tests {
    use super::*;
    #[test]
    fn title_recovered_from_current_and_stopped_resume_sources() {
        let directory = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
        db.conn().execute("INSERT INTO instances(name,tool,session_id,created_at,launch_context) VALUES('lobo','codex','session',10,?1)", [json!({"title":"native owner"}).to_string()]).unwrap();
        let row = db.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!(snapshot(&row).unwrap().title.as_deref(), Some("native owner"));
        let saved = db.get_instance_snapshot("lobo").unwrap().unwrap();
        db.log_event("life", "lobo", &json!({"action":"stopped","snapshot":saved})).unwrap();
        assert_eq!(stopped(&db, "lobo", None).unwrap().unwrap().1.title.as_deref(), Some("native owner"));
    }
}
