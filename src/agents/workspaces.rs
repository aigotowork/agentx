//! Frozen collaboration membership and native workspace endpoint policy.
//! Workspace membership is adopted launch data; cwd and role origin do not
//! confer membership or participant authority.

use super::attachments::ParticipantRef;
use crate::config::roles::ResolvedRole;
use crate::db::{AgentxDb, InstanceRow};
use crate::service::{ServiceError, ServiceResult};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct LaunchCollaboration {
    pub workspace: Option<String>,
    pub role: Option<ResolvedRole>,
    pub endpoint: bool,
    /// Raw user-authored fixed additions, separate from per-incarnation identity text.
    #[serde(default)]
    pub instruction_additions: Option<String>,
}

/// Return the adopted snapshot, retaining legacy rows as unassigned/global.
pub(crate) fn collaboration(row: &InstanceRow) -> Option<LaunchCollaboration> {
    collaboration_from_context(row.launch_context.as_deref())
}

fn collaboration_from_context(raw: Option<&str>) -> Option<LaunchCollaboration> {
    let context: serde_json::Value = serde_json::from_str(raw?).ok()?;
    serde_json::from_value(context.get("collaboration")?.clone()).ok()
}

pub(crate) fn workspace(row: &InstanceRow) -> Option<String> {
    workspace_from_context(row.launch_context.as_deref())
}

pub(crate) fn workspace_from_context(raw: Option<&str>) -> Option<String> {
    let context: serde_json::Value = serde_json::from_str(raw?).ok()?;
    context.get("collaboration")?.get("workspace")?.as_str().map(str::to_owned)
}

/// Non-mutating native policy result; participant is always an exact incarnation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct EndpointInspection {
    pub status: EndpointStatus,
    pub participant: Option<ParticipantRef>,
    pub candidates: u32,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EndpointStatus {
    Bound,
    Adoptable,
    Absent,
    Ambiguous,
    RoleChanged,
    Stale,
    Unknown,
}

pub(crate) enum CheckedCollaboration {
    Missing,
    Valid(LaunchCollaboration),
    Unavailable(String),
}

pub(crate) const GLOBAL_WORKSPACE_KEY: &str = "global";

/// Member queries accept a saved key, without consulting today's filesystem.
pub(crate) fn is_workspace_key(key: &str) -> bool {
    key == GLOBAL_WORKSPACE_KEY || saved_absolute(key)
}

/// Exact membership, unlike CLI visibility: unknown and known global differ.
pub(crate) fn adopted_workspace_key(context: Option<&str>) -> Option<String> {
    match checked_collaboration(context) {
        CheckedCollaboration::Valid(snapshot) => Some(snapshot.workspace.unwrap_or_else(|| GLOBAL_WORKSPACE_KEY.into())),
        CheckedCollaboration::Missing | CheckedCollaboration::Unavailable(_) => None,
    }
}

pub(crate) fn checked_collaboration(context: Option<&str>) -> CheckedCollaboration {
    let Some(context) = context else { return CheckedCollaboration::Missing };
    let Ok(context) = serde_json::from_str::<serde_json::Value>(context) else {
        return CheckedCollaboration::Unavailable("invalid saved launch context".into());
    };
    checked_collaboration_value(&context)
}

/// Decode without touching current files: adopted paths can outlive their source.
pub(crate) fn checked_collaboration_value(context: &serde_json::Value) -> CheckedCollaboration {
    if !context.is_object() {
        return CheckedCollaboration::Unavailable("saved launch context is not an object".into());
    }
    let Some(value) = context.get("collaboration") else { return CheckedCollaboration::Missing };
    let Some(object) = value.as_object() else { return CheckedCollaboration::Unavailable("invalid saved collaboration".into()) };
    if ["workspace", "role", "endpoint"].iter().any(|key| !object.contains_key(*key)) || object.keys().any(|key| !["workspace", "role", "endpoint", "instruction_additions"].contains(&key.as_str())) {
        return CheckedCollaboration::Unavailable("unsupported saved collaboration shape".into());
    }
    let Ok(snapshot) = serde_json::from_value::<LaunchCollaboration>(value.clone()) else {
        return CheckedCollaboration::Unavailable("invalid saved collaboration fields".into());
    };
    if snapshot.workspace.as_deref().is_some_and(|path| !saved_absolute(path)) {
        return CheckedCollaboration::Unavailable("saved workspace is not an absolute normalized path".into());
    }
    if snapshot.endpoint && snapshot.role.is_none() {
        return CheckedCollaboration::Unavailable("saved endpoint has no adopted role".into());
    }
    if let Some(role) = &snapshot.role {
        let source = role.namespace.split_once(':');
        if !saved_absolute(&role.path.to_string_lossy()) || !source.is_some_and(|(origin, path)| matches!(origin, "local" | "global") && saved_absolute(path)) || role.name.is_empty() || role.profile.is_empty() {
            return CheckedCollaboration::Unavailable("invalid saved role source or identity".into());
        }
    }
    CheckedCollaboration::Valid(snapshot)
}

fn saved_absolute(value: &str) -> bool {
    let path = std::path::Path::new(value);
    path.is_absolute() && !value.contains('\0') && !path.components().any(|part| matches!(part, std::path::Component::ParentDir | std::path::Component::CurDir))
}

pub(crate) fn endpoint_inspect(db: &AgentxDb, workspace: Option<&str>, role_path: &str) -> ServiceResult<EndpointInspection> {
    let key = endpoint_key(workspace)?;
    if !std::path::Path::new(role_path).is_absolute() {
        return Err(ServiceError::InvalidInput("endpoint role path must be absolute".into()));
    }
    let result = |status, participant, candidates, reason: Option<&str>| EndpointInspection { status, participant, candidates, reason: reason.map(str::to_owned) };
    if let Some(binding) = endpoint_binding(db, &key)? {
        if binding.role_path != role_path {
            return Ok(result(EndpointStatus::RoleChanged, Some(binding.participant), 0, Some("endpoint role changed; exact retirement and explicit reset required")));
        }
        if !retired(db, &binding.participant)? {
            let Some(row) = db.get_instance_full(&binding.participant.canonical_name).map_err(internal)? else {
                return Ok(result(EndpointStatus::Unknown, Some(binding.participant), 0, Some("endpoint disappeared without exact retirement evidence")));
            };
            if participant(&row) != binding.participant {
                return Ok(result(EndpointStatus::Stale, Some(binding.participant), 0, Some("namesake participant does not match bound incarnation")));
            }
            if !compatible(&row, workspace, role_path) {
                return Ok(result(EndpointStatus::RoleChanged, Some(binding.participant), 0, Some("adopted membership or role no longer matches binding")));
            }
            return Ok(result(EndpointStatus::Bound, Some(binding.participant), 1, None));
        }
    }
    let mut count = 0u32;
    let mut only = None;
    for row in db.iter_instances_full().map_err(internal)? {
        if compatible(&row, workspace, role_path) && !retired(db, &participant(&row))? {
            count = count.saturating_add(1);
            if count == 1 {
                only = Some(participant(&row));
            } else {
                only = None;
            }
        }
    }
    Ok(match count {
        0 => result(EndpointStatus::Absent, None, 0, None),
        1 => result(EndpointStatus::Adoptable, only, 1, None),
        _ => result(EndpointStatus::Ambiguous, None, count, Some("multiple compatible participants; use an explicit recipient")),
    })
}

#[derive(serde::Serialize, serde::Deserialize)]
struct EndpointBinding {
    role_path: String,
    participant: ParticipantRef,
}

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

fn endpoint_key(workspace: Option<&str>) -> ServiceResult<String> {
    if workspace.is_some_and(|path| !std::path::Path::new(path).is_absolute()) {
        return Err(ServiceError::InvalidInput("workspace must be an absolute canonical path".into()));
    }
    Ok(format!("workspace_endpoint:{}", serde_json::to_string(&workspace).map_err(internal)?))
}

fn endpoint_binding(db: &AgentxDb, key: &str) -> ServiceResult<Option<EndpointBinding>> {
    db.kv_get(key).map_err(internal)?.map(|value| serde_json::from_str(&value).map_err(internal)).transpose()
}

fn participant(row: &InstanceRow) -> ParticipantRef {
    ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() }
}

/// Retirement is an exact native stopped record, never absence or a timeout.
/// A retained row must additionally be the soft-retired, process-free record.
fn retired(db: &AgentxDb, expected: &ParticipantRef) -> ServiceResult<bool> {
    if let Some(row) = db.get_instance_full(&expected.canonical_name).map_err(internal)?
        && participant(&row) == *expected
        && (row.status != "inactive" || !row.status_context.starts_with("exit:") || row.pid.is_some())
    {
        return Ok(false);
    }
    let mut statement = db.conn().prepare("SELECT json_extract(data, '$.snapshot.created_at') FROM events WHERE type='life' AND instance=?1 AND json_extract(data, '$.action')='stopped'").map_err(internal)?;
    let values = statement.query_map([&expected.canonical_name], |row| row.get::<_, Option<f64>>(0)).map_err(internal)?;
    for value in values {
        if value.map_err(internal)?.is_some_and(|created| created.to_bits() == expected.incarnation_bits) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn require_write_scope(db: &AgentxDb) -> ServiceResult<()> {
    if db.conn().is_autocommit() {
        return Err(ServiceError::Internal("workspace endpoint requires native write scope".into()));
    }
    Ok(())
}

/// Resolve/adopt only within the caller's native transaction. Errors and
/// unresolved launches remain occupied endpoints; no OS effects run here.
pub(super) fn endpoint_lookup(db: &AgentxDb, workspace: Option<&str>, role_path: &str) -> ServiceResult<Option<ParticipantRef>> {
    require_write_scope(db)?;
    if !std::path::Path::new(role_path).is_absolute() {
        return Err(ServiceError::InvalidInput("endpoint role path must be absolute".into()));
    }
    let key = endpoint_key(workspace)?;
    if let Some(binding) = endpoint_binding(db, &key)? {
        if binding.role_path != role_path {
            return Err(ServiceError::Conflict("workspace endpoint role changed; retire the existing participant and explicitly reset its endpoint".into()));
        }
        if !retired(db, &binding.participant)? {
            let row = db.get_instance_full(&binding.participant.canonical_name).map_err(internal)?.ok_or_else(|| ServiceError::OutcomeUnknown("workspace endpoint disappeared without exact retirement evidence".into()))?;
            if participant(&row) != binding.participant {
                return Err(ServiceError::StaleFence);
            }
            if !compatible(&row, workspace, role_path) {
                return Err(ServiceError::Conflict("workspace endpoint membership or adopted role no longer matches its binding".into()));
            }
            return Ok(Some(binding.participant));
        }
    }
    let mut candidates = Vec::new();
    for row in db.iter_instances_full().map_err(internal)? {
        if compatible(&row, workspace, role_path) && !retired(db, &participant(&row))? {
            candidates.push(participant(&row));
        }
    }
    match candidates.as_slice() {
        [] => Ok(None),
        [only] => {
            bind_endpoint(db, workspace, role_path, only)?;
            Ok(Some(only.clone()))
        }
        _ => Err(ServiceError::Conflict(format!("workspace endpoint is ambiguous: {} compatible participants; select an explicit recipient", candidates.len()))),
    }
}

fn compatible(row: &InstanceRow, workspace: Option<&str>, role_path: &str) -> bool {
    !super::records::is_remote_instance(row) && collaboration(row).is_some_and(|snapshot| snapshot.workspace.as_deref() == workspace && snapshot.role.is_some_and(|role| role.path == std::path::Path::new(role_path)))
}

pub(super) fn bind_endpoint(db: &AgentxDb, workspace: Option<&str>, role_path: &str, participant: &ParticipantRef) -> ServiceResult<()> {
    require_write_scope(db)?;
    let value = EndpointBinding { role_path: role_path.into(), participant: participant.clone() };
    db.kv_set(&endpoint_key(workspace)?, Some(&serde_json::to_string(&value).map_err(internal)?)).map_err(internal)
}

pub(super) fn is_bound_endpoint(db: &AgentxDb, row: &InstanceRow) -> ServiceResult<bool> {
    let Some(snapshot) = collaboration(row) else { return Ok(false) };
    Ok(endpoint_binding(db, &endpoint_key(snapshot.workspace.as_deref())?)?.is_some_and(|binding| binding.participant == participant(row)))
}

pub(super) fn protect_replacement(db: &AgentxDb, row: &InstanceRow) -> ServiceResult<()> {
    if is_bound_endpoint(db, row)? && !retired(db, &participant(row))? {
        return Err(ServiceError::Conflict("workspace endpoint still owns this participant; confirm retirement before replacing it".into()));
    }
    Ok(())
}

/// Session binding may move an already reserved launch to its canonical row.
/// Only that existing native binding transaction can transfer the endpoint.
pub(super) fn migrate_endpoint(db: &AgentxDb, previous: &InstanceRow, current: &InstanceRow) -> ServiceResult<()> {
    require_write_scope(db)?;
    if participant(previous) == participant(current) || !is_bound_endpoint(db, previous)? {
        return Ok(());
    }
    let snapshot = collaboration(previous).ok_or(ServiceError::StaleFence)?;
    let role = snapshot.role.as_ref().ok_or(ServiceError::StaleFence)?;
    if !compatible(current, snapshot.workspace.as_deref(), &role.path.to_string_lossy()) {
        return Err(ServiceError::Conflict("canonical session has different workspace or role than its reserved endpoint".into()));
    }
    if let Some(old) = db.get_instance_full(&previous.name).map_err(internal)?
        && (participant(&old) != participant(previous) || old.pid.is_some() || old.status != "inactive" || !old.status_context.starts_with("exit:"))
    {
        return Err(ServiceError::StaleFence);
    }
    bind_endpoint(db, snapshot.workspace.as_deref(), &role.path.to_string_lossy(), &participant(current))
}

pub(super) fn validate(snapshot: &LaunchCollaboration) -> ServiceResult<()> {
    endpoint_key(snapshot.workspace.as_deref())?;
    if snapshot.endpoint && snapshot.role.is_none() {
        return Err(ServiceError::InvalidInput("workspace endpoint requires an adopted role".into()));
    }
    if snapshot.role.as_ref().is_some_and(|role| !role.path.is_absolute()) {
        return Err(ServiceError::InvalidInput("adopted role path must be absolute".into()));
    }
    Ok(())
}

pub(super) fn endpoint_reset(db: &AgentxDb, workspace: Option<&str>) -> ServiceResult<()> {
    require_write_scope(db)?;
    let key = endpoint_key(workspace)?;
    if let Some(binding) = endpoint_binding(db, &key)? {
        if !retired(db, &binding.participant)? {
            return Err(ServiceError::Conflict("workspace endpoint is not definitively retired; live, failed and uncertain endpoints cannot be reset".into()));
        }
        db.kv_set(&key, None).map_err(internal)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::launch_service::{Request, Reservation, Reserve, Response};
    use crate::agents::launch_store::handle;
    use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestContext, RequestId};
    use serde_json::json;

    fn fixture() -> (tempfile::TempDir, AgentxDb, RequestContext) {
        let directory = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&directory.path().join("agentx.db")).unwrap();
        let pid = std::process::id();
        let context = RequestContext {
            caller: CallerContext { request_id: RequestId("membership".into()), invocation: InvocationContext::default(), deadline_unix_ms: None, expected_database_generation: None },
            process: CallerProcess { pid, incarnation: crate::sys::process::identity(pid) },
            daemon_instance: "membership-fixture".into(),
            database_generation: db.database_generation().unwrap(),
        };
        (directory, db, context)
    }

    fn snapshot(workspace: Option<&str>, endpoint: bool) -> LaunchCollaboration {
        LaunchCollaboration {
            workspace: workspace.map(str::to_owned),
            role: Some(ResolvedRole { name: "controller".into(), path: "/roles/controller.toml".into(), namespace: "global:/roles".into(), profile: "deep".into(), prompt: "Frozen instructions".into(), description: None, default: true }),
            endpoint,
            instruction_additions: None,
        }
    }

    fn input(name: &str, collaboration: Option<LaunchCollaboration>) -> Reserve {
        Reserve {
            name: Some(name.into()),
            tool: "codex".into(),
            directory: "/fixture".into(),
            process_binding: format!("process-{name}"),
            prior_session: None,
            resume_cursor: None,
            background: false,
            launch_args: vec![],
            launch_profile: None,
            collaboration,
            title: None,
        labels: Default::default(),
        }
    }

    fn request(context: &RequestContext, id: &str) -> RequestContext {
        let mut next = context.clone();
        next.caller.request_id = RequestId(id.into());
        next
    }

    fn reserve_member(db: &AgentxDb, context: &RequestContext, name: &str, collaboration: Option<LaunchCollaboration>) -> Reservation {
        match handle(db, &request(context, name), Request::Reserve(input(name, collaboration))).unwrap() {
            Response::Reserved(value) => value,
            _ => panic!("reservation response"),
        }
    }

    fn lookup(db: &AgentxDb, context: &RequestContext, workspace: Option<&str>) -> ServiceResult<Option<ParticipantRef>> {
        match handle(db, context, Request::EndpointLookup { workspace: workspace.map(str::to_owned), role_path: "/roles/controller.toml".into() })? {
            Response::Endpoint(value) => Ok(value),
            _ => panic!("endpoint response"),
        }
    }

    fn stop(db: &AgentxDb, name: &str) {
        let row = db.get_instance_full(name).unwrap().unwrap();
        let event = json!({"action":"stopped","reason":"fixture","snapshot":db.get_instance_snapshot(name).unwrap().unwrap()});
        assert!(db.finalize_instance_stop(name, row.created_at, row.session_id.as_deref(), row.agent_id.as_deref(), &event).unwrap());
    }

    #[test]
    fn collaboration_survives_terminal_recapture_and_stopped_snapshot() {
        let (_directory, db, mut context) = fixture();
        let adopted = snapshot(Some("/workspace"), false);
        context.caller.invocation.launch_context = Some(json!({"pane_id":"pane-7","audit":"retained"}));
        let reserved = reserve_member(&db, &context, "lobo", Some(adopted.clone()));
        handle(&db, &context, Request::TerminalLaunched { reservation: reserved, requested: None, effective: "fixture".into(), background: None }).unwrap();
        context.caller.invocation.cwd = "/different".into();
        context.caller.invocation.launch_context = Some(json!({"env":{"TERM":"xterm"},"collaboration":snapshot(Some("/wrong"), false)}));
        crate::agents::invocation_binding::capture_launch_context(&db, &context, "lobo").unwrap();
        let row = db.get_instance_full("lobo").unwrap().unwrap();
        assert_eq!(collaboration(&row), Some(adopted.clone()));
        assert_eq!(workspace(&row).as_deref(), Some("/workspace"));
        let context: serde_json::Value = serde_json::from_str(row.launch_context.as_deref().unwrap()).unwrap();
        assert_eq!(context["pane_id"], "pane-7");
        assert_eq!(context["audit"], "retained");
        assert_eq!(context["terminal_preset_effective"], "fixture");
        let saved = db.get_instance_snapshot("lobo").unwrap().unwrap();
        let saved_context: serde_json::Value = serde_json::from_str(saved["launch_context"].as_str().unwrap()).unwrap();
        assert_eq!(saved_context["collaboration"], serde_json::to_value(adopted).unwrap());
    }

    #[test]
    fn legacy_explicit_global_and_roleless_local_membership_remain_distinct() {
        let (_directory, db, context) = fixture();
        reserve_member(&db, &context, "lobo", None);
        reserve_member(&db, &context, "mora", Some(LaunchCollaboration { workspace: None, role: None, endpoint: false, instruction_additions: None }));
        reserve_member(&db, &context, "riva", Some(LaunchCollaboration { workspace: Some("/workspace".into()), role: None, endpoint: false, instruction_additions: None }));
        assert!(collaboration(&db.get_instance_full("lobo").unwrap().unwrap()).is_none());
        assert!(collaboration(&db.get_instance_full("mora").unwrap().unwrap()).is_some());
        assert_eq!(workspace(&db.get_instance_full("riva").unwrap().unwrap()).as_deref(), Some("/workspace"));
        assert!(lookup(&db, &context, Some("/workspace")).unwrap().is_none());
    }

    #[test]
    fn pane_title_uses_adopted_role_and_launch_title() {
        let (_directory, db, context) = fixture();
        let mut launch = input("saki", Some(snapshot(Some("/workspace"), false)));
        launch.title = Some("优化终端标题".into());
        handle(&db, &request(&context, "pane-title"), Request::Reserve(launch)).unwrap();
        let row = db.get_instance_full("saki").unwrap().unwrap();
        let display = crate::agents::launch_title::pane_display_from_record(&row);
        assert_eq!(display, "saki [controller] · 优化终端标题");
        assert_eq!(crate::shared::format_pane_title("active", &display, "codex"), format!("{} {display} [codex]", crate::shared::status_icon("active")));
        assert_eq!(crate::shared::format_pane_title_combined("blocked", &display, Some("Working")), format!("{} {display} - Working", crate::shared::status_icon("blocked")));

        // Historical rows without adopted data must retain their original identity.
        let mut legacy = row;
        for context in [None, Some("{}".into()), Some("invalid json".into())] {
            legacy.launch_context = context;
            assert_eq!(crate::agents::launch_title::pane_display_from_record(&legacy), "saki");
        }
    }

    #[test]
    fn adopts_only_one_compatible_member_and_preserves_adopted_prompt() {
        let (_directory, db, context) = fixture();
        let local = reserve_member(&db, &context, "lobo", Some(snapshot(Some("/workspace"), false)));
        reserve_member(&db, &context, "mora", Some(snapshot(None, false)));
        assert_eq!(lookup(&db, &context, Some("/workspace")).unwrap(), Some(local.participant.clone()));
        db.update_instance_fields("lobo", json!({"status":"active","directory":"/moved"}).as_object().unwrap()).unwrap();
        assert_eq!(lookup(&db, &context, Some("/workspace")).unwrap(), Some(local.participant));
        assert_eq!(collaboration(&db.get_instance_full("lobo").unwrap().unwrap()).unwrap().role.unwrap().prompt, "Frozen instructions");
    }

    #[test]
    fn ambiguous_members_do_not_create_an_endpoint_binding() {
        let (_directory, db, context) = fixture();
        for name in ["lobo", "mora"] {
            reserve_member(&db, &context, name, Some(snapshot(Some("/workspace"), false)));
        }
        assert!(matches!(lookup(&db, &context, Some("/workspace")), Err(ServiceError::Conflict(_))));
        assert!(db.kv_get(&endpoint_key(Some("/workspace")).unwrap()).unwrap().is_none());
    }

    #[test]
    fn concurrent_reservations_have_one_winner_and_retained_replay() {
        let (directory, db, context) = fixture();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let workers: Vec<_> = ["lobo", "mora"]
            .into_iter()
            .map(|name| {
                let path = directory.path().join("agentx.db");
                let context = request(&context, name);
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let db = AgentxDb::open_at(&path).unwrap();
                    barrier.wait();
                    let input = input(name, Some(snapshot(Some("/workspace"), true)));
                    let outcome = handle(&db, &context, Request::Reserve(input.clone()));
                    if let Ok(Response::Reserved(reserved)) = &outcome {
                        let Response::Reserved(replayed) = handle(&db, &context, Request::Reserve(input)).unwrap() else { panic!("replay response") };
                        assert_eq!(*reserved, replayed);
                    }
                    outcome
                })
            })
            .collect();
        let outcomes: Vec<_> = workers.into_iter().map(|worker| worker.join().unwrap()).collect();
        assert_eq!(outcomes.iter().filter(|outcome| matches!(outcome, Ok(Response::Reserved(_)))).count(), 1);
        assert_eq!(outcomes.iter().filter(|outcome| matches!(outcome, Err(ServiceError::Conflict(_)))).count(), 1);
        assert_eq!(db.iter_instances_full().unwrap().len(), 1);
        assert!(lookup(&db, &context, Some("/workspace")).unwrap().is_some());
    }

    #[test]
    fn endpoint_write_failure_rolls_back_identity_binding_and_request_receipt() {
        let (_directory, db, context) = fixture();
        db.conn().execute_batch("CREATE TRIGGER endpoint_failure BEFORE INSERT ON kv WHEN NEW.key LIKE 'workspace_endpoint:%' BEGIN SELECT RAISE(ABORT, 'endpoint write failure'); END;").unwrap();
        let launch = input("lobo", Some(snapshot(Some("/workspace"), true)));
        assert!(handle(&db, &context, Request::Reserve(launch.clone())).is_err());
        assert!(db.get_instance_full("lobo").unwrap().is_none());
        assert!(db.get_process_binding("process-lobo").unwrap().is_none());
        assert!(db.kv_get(&endpoint_key(Some("/workspace")).unwrap()).unwrap().is_none());
        db.conn().execute_batch("DROP TRIGGER endpoint_failure").unwrap();
        assert!(matches!(handle(&db, &context, Request::Reserve(launch)), Ok(Response::Reserved(_))));
    }

    #[test]
    fn pending_and_failed_endpoints_remain_reserved_and_cannot_be_reset_or_replaced() {
        let (_directory, db, context) = fixture();
        let reserved = reserve_member(&db, &context, "lobo", Some(snapshot(Some("/workspace"), true)));
        assert_eq!(lookup(&db, &context, Some("/workspace")).unwrap(), Some(reserved.participant.clone()));
        handle(&db, &context, Request::CancelFailed(reserved.clone())).unwrap();
        assert_eq!(lookup(&db, &context, Some("/workspace")).unwrap(), Some(reserved.participant));
        assert_eq!(db.get_instance_full("lobo").unwrap().unwrap().status_context, "launch_failed");
        assert!(matches!(handle(&db, &context, Request::EndpointReset { workspace: Some("/workspace".into()) }), Err(ServiceError::Conflict(_))));
        assert!(matches!(handle(&db, &request(&context, "replacement"), Request::Reserve(input("lobo", None))), Err(ServiceError::Conflict(_))));
        assert!(matches!(handle(&db, &request(&context, "second"), Request::Reserve(input("mora", Some(snapshot(Some("/workspace"), true))))), Err(ServiceError::Conflict(_))));
    }

    #[test]
    fn absent_row_and_namesake_without_exact_retirement_do_not_release_binding() {
        let (_directory, db, context) = fixture();
        let reserved = reserve_member(&db, &context, "lobo", Some(snapshot(Some("/workspace"), true)));
        db.delete_instance("lobo").unwrap();
        assert!(matches!(lookup(&db, &context, Some("/workspace")), Err(ServiceError::OutcomeUnknown(_))));
        db.log_event("life", "lobo", &json!({"action":"stopped","snapshot":{"created_at":0.0}})).unwrap();
        assert!(matches!(lookup(&db, &context, Some("/workspace")), Err(ServiceError::OutcomeUnknown(_))));
        db.save_instance_named("lobo", json!({"tool":"codex","created_at":f64::from_bits(reserved.participant.incarnation_bits) + 1.0}).as_object().unwrap()).unwrap();
        assert!(matches!(lookup(&db, &context, Some("/workspace")), Err(ServiceError::StaleFence)));
        assert!(matches!(handle(&db, &context, Request::EndpointReset { workspace: Some("/workspace".into()) }), Err(ServiceError::Conflict(_))));
    }

    #[test]
    fn exact_retirement_allows_same_role_successor_but_role_drift_needs_reset() {
        let (_directory, db, context) = fixture();
        reserve_member(&db, &context, "lobo", Some(snapshot(Some("/workspace"), true)));
        stop(&db, "lobo");
        assert!(lookup(&db, &context, Some("/workspace")).unwrap().is_none());
        let successor = reserve_member(&db, &context, "mora", Some(snapshot(Some("/workspace"), true)));
        assert_eq!(lookup(&db, &context, Some("/workspace")).unwrap(), Some(successor.participant));
        stop(&db, "mora");
        let changed = Request::EndpointLookup { workspace: Some("/workspace".into()), role_path: "/roles/reviewer.toml".into() };
        assert!(matches!(handle(&db, &context, changed), Err(ServiceError::Conflict(_))));
        let reset_context = request(&context, "explicit-reset");
        handle(&db, &reset_context, Request::EndpointReset { workspace: Some("/workspace".into()) }).unwrap();
        let mut changed = snapshot(Some("/workspace"), true);
        changed.role.as_mut().unwrap().path = "/roles/reviewer.toml".into();
        let current = reserve_member(&db, &context, "riva", Some(changed));
        handle(&db, &reset_context, Request::EndpointReset { workspace: Some("/workspace".into()) }).unwrap();
        let stored = endpoint_binding(&db, &endpoint_key(Some("/workspace")).unwrap()).unwrap().unwrap();
        assert_eq!(stored.participant, current.participant, "replayed reset cannot clear a later endpoint");
    }

    // Snapshot the entire logical database plus SQLite's mutation counter. A pure
    // inspect must neither mutate then restore data nor create request receipts.
    fn database_frontier(db: &AgentxDb) -> (i64, String) {
        let changes: i64 = db.conn().query_row("SELECT total_changes()", [], |row| row.get(0)).unwrap();
        let mut dump = String::new();
        for table in ["instances", "events", "kv"] {
            let mut statement = db.conn().prepare(&format!("SELECT * FROM {table} ORDER BY rowid")).unwrap();
            let columns = statement.column_count();
            let mut rows = statement.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                for index in 0..columns {
                    dump.push_str(&format!("{:?}|", row.get_ref(index).unwrap()));
                }
                dump.push('\n');
            }
        }
        (changes, dump)
    }

    fn inspect(db: &AgentxDb, context: &RequestContext, role: &str) -> EndpointInspection {
        let before = database_frontier(db);
        let Response::EndpointInspected(inspection) = handle(db, context, Request::EndpointInspect { workspace: Some("/workspace".into()), role_path: role.into() }).unwrap() else { panic!("inspection response") };
        assert_eq!(database_frontier(db), before, "inspection must preserve rows, binding, events and mutation frontier");
        inspection
    }

    #[test]
    fn inspect_absent_unique_and_ambiguous_without_adopting() {
        let (_directory, db, context) = fixture();
        assert_eq!(inspect(&db, &context, "/roles/controller.toml").status, EndpointStatus::Absent);
        let member = reserve_member(&db, &context, "lobo", Some(snapshot(Some("/workspace"), false)));
        let unique = inspect(&db, &context, "/roles/controller.toml");
        assert_eq!(unique.status, EndpointStatus::Adoptable);
        assert_eq!(unique.participant, Some(member.participant));
        assert_eq!(unique.candidates, 1);
        reserve_member(&db, &context, "mora", Some(snapshot(Some("/workspace"), false)));
        let ambiguous = inspect(&db, &context, "/roles/controller.toml");
        assert_eq!(ambiguous.status, EndpointStatus::Ambiguous);
        assert_eq!(ambiguous.candidates, 2);
        assert!(ambiguous.participant.is_none());
        assert!(db.kv_get(&endpoint_key(Some("/workspace")).unwrap()).unwrap().is_none());
    }

    #[test]
    fn inspect_pending_failed_and_role_drift_remain_occupied() {
        let (_directory, db, context) = fixture();
        let reserved = reserve_member(&db, &context, "lobo", Some(snapshot(Some("/workspace"), true)));
        assert_eq!(inspect(&db, &context, "/roles/controller.toml").status, EndpointStatus::Bound);
        handle(&db, &context, Request::CancelFailed(reserved.clone())).unwrap();
        let failed = inspect(&db, &context, "/roles/controller.toml");
        assert_eq!(failed.status, EndpointStatus::Bound);
        assert_eq!(failed.participant, Some(reserved.participant.clone()));
        let changed = inspect(&db, &context, "/roles/reviewer.toml");
        assert_eq!(changed.status, EndpointStatus::RoleChanged);
        assert_eq!(changed.participant, Some(reserved.participant));
        let mut changed_adoption = snapshot(Some("/other"), true);
        changed_adoption.role.as_mut().unwrap().path = "/roles/reviewer.toml".into();
        db.update_instance_fields("lobo", json!({"launch_context":json!({"collaboration":changed_adoption}).to_string()}).as_object().unwrap()).unwrap();
        assert_eq!(inspect(&db, &context, "/roles/controller.toml").status, EndpointStatus::RoleChanged);
    }

    #[test]
    fn inspect_absent_bound_row_is_unknown_and_namesake_is_stale() {
        let (_directory, db, context) = fixture();
        let reserved = reserve_member(&db, &context, "lobo", Some(snapshot(Some("/workspace"), true)));
        db.delete_instance("lobo").unwrap();
        let unknown = inspect(&db, &context, "/roles/controller.toml");
        assert_eq!(unknown.status, EndpointStatus::Unknown);
        assert_eq!(unknown.participant, Some(reserved.participant.clone()));
        db.log_event("life", "lobo", &json!({"action":"stopped","snapshot":{"created_at":0.0}})).unwrap();
        assert_eq!(inspect(&db, &context, "/roles/controller.toml").status, EndpointStatus::Unknown);
        db.save_instance_named("lobo", json!({"tool":"codex","created_at":f64::from_bits(reserved.participant.incarnation_bits)+1.0}).as_object().unwrap()).unwrap();
        assert_eq!(inspect(&db, &context, "/roles/controller.toml").status, EndpointStatus::Stale);
    }

    #[test]
    fn inspect_retired_binding_observes_successor_without_rebinding() {
        let (_directory, db, context) = fixture();
        reserve_member(&db, &context, "lobo", Some(snapshot(Some("/workspace"), true)));
        stop(&db, "lobo");
        assert_eq!(inspect(&db, &context, "/roles/controller.toml").status, EndpointStatus::Absent);
        let next = reserve_member(&db, &context, "mora", Some(snapshot(Some("/workspace"), false)));
        let candidate = inspect(&db, &context, "/roles/controller.toml");
        assert_eq!(candidate.status, EndpointStatus::Adoptable);
        assert_eq!(candidate.participant, Some(next.participant));
        assert_eq!(endpoint_binding(&db, &endpoint_key(Some("/workspace")).unwrap()).unwrap().unwrap().participant.canonical_name, "lobo");
        assert_eq!(inspect(&db, &context, "/roles/reviewer.toml").status, EndpointStatus::RoleChanged);
    }

    #[test]
    fn checked_decoder_rejects_invalid_membership_without_changing_legacy_helper() {
        let (_directory, db, context) = fixture();
        reserve_member(&db, &context, "lobo", None);
        for value in [json!({"collaboration":null}), json!({"collaboration":{"endpoint":false}}), json!([])] {
            let raw = value.to_string();
            assert!(matches!(checked_collaboration(Some(&raw)), CheckedCollaboration::Unavailable(_)));
        }
        assert!(matches!(checked_collaboration(None), CheckedCollaboration::Missing));
        assert!(matches!(checked_collaboration(Some("{}")), CheckedCollaboration::Missing));
        assert!(matches!(checked_collaboration(Some("{")), CheckedCollaboration::Unavailable(_)));
        for path in ["relative", "/workspace/../other"] {
            let adopted = snapshot(Some(path), false);
            let raw = json!({"collaboration":adopted}).to_string();
            assert!(matches!(checked_collaboration(Some(&raw)), CheckedCollaboration::Unavailable(_)));
            db.update_instance_fields("lobo", json!({"launch_context":raw}).as_object().unwrap()).unwrap();
            assert!(collaboration(&db.get_instance_full("lobo").unwrap().unwrap()).is_some(), "legacy helper stays permissive");
        }
        let mut value = serde_json::to_value(snapshot(None, false)).unwrap();
        value["future_version"] = json!(2);
        assert!(matches!(checked_collaboration(Some(&json!({"collaboration":value}).to_string())), CheckedCollaboration::Unavailable(_)));
    }
}
