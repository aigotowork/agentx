//! Explicit invocation adapter for the canonical process/session binding policy.
//! Launch metadata is captured by the originating process, never by the daemon.

use super::attachments::ParticipantRef;
use super::binding::{self, BindingContext, ToolCheckedBind};
use super::service::ResolvedActor;
use crate::db::AgentxDb;
use crate::service::{RequestContext, ServiceError, ServiceResult};

fn internal(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Internal(error.to_string())
}

pub(crate) fn bind_session(db: &AgentxDb, context: &RequestContext, session: &str, tool: &str, recover_soft_stopped: bool) -> ServiceResult<Option<ResolvedActor>> {
    bind_session_policy(db, context, session, tool, false, recover_soft_stopped)
}

pub(crate) fn bind_session_with_creation(db: &AgentxDb, context: &RequestContext, session: &str, tool: &str, create_if_unbound: bool) -> ServiceResult<Option<ResolvedActor>> {
    if create_if_unbound && context.caller.invocation.process_binding_id.as_deref().is_none_or(str::is_empty) {
        return Err(ServiceError::IdentityRequired("process binding required to create a hooked session identity".into()));
    }
    bind_session_policy(db, context, session, tool, create_if_unbound, false)
}

fn bind_session_policy(db: &AgentxDb, context: &RequestContext, session: &str, tool: &str, create_if_unbound: bool, recover_soft_stopped: bool) -> ServiceResult<Option<ResolvedActor>> {
    if session.is_empty() || tool.is_empty() {
        return Err(ServiceError::InvalidInput("session and tool required".into()));
    }
    let invocation = &context.caller.invocation;
    let effective = BindingContext::explicit(db, invocation);
    db.with_write_scope(|| {
        let process = invocation.process_binding_id.as_deref().filter(|value| !value.is_empty());
        let previous = process.map(|process| db.get_process_binding(process)).transpose()?.flatten().map(|name| db.get_instance_full(&name)).transpose()?.flatten();
        // Launch reserves this binding before any Provider callback. Losing it
        // means retirement, not permission for a late hook to create an Agent.
        // Keep explicit soft-stop recovery separate from unbound hook creation.
        if previous.as_ref().is_some_and(|row| row.status_context == "exit:killed") || (!recover_soft_stopped && invocation.environment.get("AGENTX_LAUNCHED").map(String::as_str) == Some("1") && previous.is_none()) {
            return Err(ServiceError::StaleFence.into());
        }
        let mut name = match binding::bind_session_for_tool_with_context(db, &effective, session, process, tool, create_if_unbound) {
            ToolCheckedBind::Bound(name) => Some(name),
            ToolCheckedBind::Unbound => None,
            ToolCheckedBind::Rejected => return Err(ServiceError::Forbidden("session/process belongs to another tool or binding migration failed".into()).into()),
        };
        if name.is_none()
            && recover_soft_stopped
            && tool == "omp"
            && let (Some(name_hint), Some(process)) = (invocation.environment.get("AGENTX_INSTANCE_NAME"), process)
        {
            // This is the existing narrowly scoped OMP recovery policy. The
            // environment name remains a claim checked against the stored row.
            name = binding::recover_process_binding_for_instance(db, name_hint, session, process);
        }
        let Some(name) = name else {
            return Ok(None);
        };
        let record = db.get_instance_full(&name)?.ok_or(ServiceError::StaleFence)?;
        if record.tool != tool {
            return Err(ServiceError::StaleFence.into());
        }
        if let Some(previous) = previous {
            super::workspaces::migrate_endpoint(db, &previous, &record)?;
        }
        Ok(Some(ResolvedActor { participant: ParticipantRef { canonical_name: name, incarnation_bits: record.created_at.to_bits() }, record }))
    })
    .map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

pub(crate) fn capture_launch_context(db: &AgentxDb, context: &RequestContext, name: &str) -> ServiceResult<()> {
    let Some(snapshot) = &context.caller.invocation.launch_context else {
        return Ok(());
    };
    let snapshot = snapshot.as_object().ok_or_else(|| ServiceError::InvalidInput("launch context must be an object".into()))?;
    db.with_write_scope(|| {
        let row = db.get_instance_full(name)?.ok_or(ServiceError::StaleFence)?;
        let mut merged: serde_json::Map<String, serde_json::Value> = row.launch_context.as_deref().map(serde_json::from_str).transpose()?.unwrap_or_default();
        for (key, value) in snapshot {
            // Recapture observes host metadata; adopted collaboration is immutable.
            if !matches!(key.as_str(), "collaboration" | "title") || !merged.contains_key(key) {
                merged.insert(key.clone(), value.clone());
            }
        }
        binding::store_launch_context(db, name, merged)
    })
    .map_err(|error| error.downcast_ref::<ServiceError>().cloned().unwrap_or_else(|| internal(error)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::{CallerContext, CallerProcess, InvocationContext, RequestId};

    fn context(environment: std::collections::HashMap<String, String>) -> RequestContext {
        RequestContext {
            caller: CallerContext {
                request_id: RequestId("fixture".into()),
                invocation: InvocationContext { cwd: "/originating/project".into(), environment, process_binding_id: Some("origin-binding".into()), ..Default::default() },
                deadline_unix_ms: None,
                expected_database_generation: None,
            },
            process: CallerProcess { pid: 123, incarnation: Some("caller-instance".into()) },
            daemon_instance: "daemon".into(),
            database_generation: "generation".into(),
        }
    }

    #[test]
    fn explicit_initializer_uses_database_scope_and_caller_environment() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.toml"), "[preferences]\ntimeout = 31\n").unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        let request = context(std::collections::HashMap::from([("AGENTX_TIMEOUT".into(), "47".into()), ("AGENTX_LAUNCHED".into(), "1".into())]));
        let effective = BindingContext::explicit(&db, &request.caller.invocation);
        assert!(binding::initialize_instance_with_context(&db, &effective, "scoped", Some("session"), None, None, None, None, Some("claude"), false, None, None, None, None));
        let row = db.get_instance_full("scoped").unwrap().unwrap();
        assert_eq!(row.directory, "/originating/project");
        assert_eq!(row.wait_timeout, Some(47));
        assert_eq!(row.pid, None, "daemon PID must not become participant PID");
    }

    #[test]
    fn plugin_tool_disagreement_does_not_change_bindings() {
        let dir = tempfile::tempdir().unwrap();
        let db = AgentxDb::open_at(&dir.path().join("agentx.db")).unwrap();
        db.conn().execute_batch("INSERT INTO instances(name,tool,created_at) VALUES('owner','claude',1);").unwrap();
        db.set_process_binding("origin-binding", "old-session", "owner").unwrap();
        let request = context(Default::default());
        assert!(matches!(bind_session(&db, &request, "new-session", "omp", true), Err(ServiceError::Forbidden(_))));
        assert_eq!(db.get_process_binding("origin-binding").unwrap().as_deref(), Some("owner"));
        assert!(db.get_session_binding("new-session").unwrap().is_none());
        assert_eq!(db.get_instance_full("owner").unwrap().unwrap().session_id, None);
    }
}
