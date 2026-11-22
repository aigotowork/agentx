//! Structured lineage selection and primary-generation routing. Provider file
//! contents are supplied as bounded envelope facts. Late launch recovery only
//! probes the retained OS process incarnation.
use super::*;
use std::collections::BTreeSet;

fn fresh(db: &AgentxDb, context: &RequestContext, binding: Option<&(Option<String>, String)>, expected: Option<&str>, native_start: bool) -> ServiceResult<bool> {
    let Some((session, name)) = binding else {
        return Ok(false);
    };
    if session.is_some() {
        return Ok(false);
    }
    let Some(row) = db.get_instance_full(name).map_err(internal)? else {
        return Ok(false);
    };
    if row.tool != "claude" || row.origin_device_id.is_some() || row.parent_name.as_deref().is_some_and(|parent| !parent.is_empty()) {
        return Ok(false);
    }
    let resumed = expected.is_some_and(|session| row.session_id.as_deref() == Some(session));
    Ok((row.session_id.is_none() || resumed)
        && (resumed
            || crate::agents::records::is_launching_placeholder(&row)
            || (row.status == "blocked" && row.status_context == "launch_blocked")
            || (row.status == "listening" && matches!(row.status_context.as_str(), "start" | "ready_observed" | "launch_blocked_cleared"))
            || (native_start && same_live_timed_out_launch(context, &row))))
}

/// A launch diagnosis can run before a slow wrapper reaches SessionStart.
/// An exact, still-live reservation can finish binding despite that diagnosis.
fn same_live_timed_out_launch(context: &RequestContext, row: &InstanceRow) -> bool {
    if row.status != "inactive" || row.status_context != "launch_failed" || row.session_id.is_some() {
        return false;
    }
    let invocation = &context.caller.invocation;
    let env = &invocation.environment;
    if env.get("AGENTX_LAUNCHED").map(String::as_str) != Some("1")
        || env.get("AGENTX_LAUNCH_GENERATION").map(String::as_str) != Some(context.database_generation.as_str())
        || env.get("AGENTX_LAUNCH_PARTICIPANT").and_then(|value| serde_json::from_str::<ParticipantRef>(value).ok()).as_ref() != Some(&participant(row))
    {
        return false;
    }
    let Some(launch) = row.launch_context.as_deref().and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok()) else { return false };
    let stored_binding = launch.get("process_id").and_then(serde_json::Value::as_str).filter(|binding| !binding.is_empty());
    if stored_binding.is_none() || stored_binding != invocation.process_binding_id.as_deref() {
        return false;
    }
    let Some(pid) = row.pid.and_then(|pid| u32::try_from(pid).ok()).filter(|pid| *pid != 0) else { return false };
    let Some(incarnation) = row.pid_identity().filter(|identity| !identity.is_empty()) else { return false };
    crate::sys::process::is_alive(pid) && crate::sys::process::identity(pid).as_deref() == Some(incarnation.as_str())
}

fn session(db: &AgentxDb, context: &RequestContext, input: &Observation) -> ServiceResult<String> {
    let invocation = &context.caller.invocation;
    if invocation.environment.get("AGENTX_IS_FORK").map(String::as_str) != Some("1") {
        return Ok(input.session_id.clone());
    }
    let Some(candidate) = input.env_session_candidate.as_deref().filter(|value| value.len() == 36 && value.chars().filter(|c| *c == '-').count() == 4) else {
        return Ok(input.session_id.clone());
    };
    let Some(process) = invocation.process_binding_id.as_deref().filter(|value| !value.is_empty()) else {
        return Ok(input.session_id.clone());
    };
    let binding = db.get_process_binding_full(process).map_err(internal)?;
    let allow = match input.kind {
        ObservationKind::SessionStart { source: StartSource::Fork | StartSource::Startup | StartSource::Resume } => fresh(db, context, binding.as_ref(), None, true)?,
        _ => binding.as_ref().and_then(|(session, _)| session.as_deref()) == Some(candidate),
    };
    Ok(if allow { candidate.to_owned() } else { input.session_id.clone() })
}

enum Lineage {
    Unknown,
    Owner(String),
    Ambiguous,
}
fn lineage(db: &AgentxDb, input: &Observation, session: &str, validated: bool) -> ServiceResult<Lineage> {
    let mut owners = BTreeSet::new();
    if let Some(path) = input.transcript_path.as_deref().filter(|path| !path.is_empty()) {
        owners.extend(db.get_instances_by_transcript_path(path).map_err(internal)?);
    }
    let mut ids = BTreeSet::new();
    if validated {
        ids.insert(session.to_owned());
    }
    match &input.evidence {
        TranscriptEvidence::ReadFailed => return Err(ServiceError::Unavailable("originating Claude transcript read failed".into())),
        TranscriptEvidence::Envelopes { top_level_sessions, message_sessions, bytes_read, records_read } => {
            if *bytes_read > 512 * 1024 || *records_read > 2048 || top_level_sessions.len() > 4096 || message_sessions.len() > 2048 || top_level_sessions.iter().chain(message_sessions).any(|id| id.is_empty() || id.len() > 1024) {
                return Err(ServiceError::InvalidInput("bounded Claude transcript envelope evidence required".into()));
            }
            ids.extend(top_level_sessions.iter().filter(|id| validated || id.as_str() != session).cloned());
            ids.extend(message_sessions.iter().cloned());
        }
        TranscriptEvidence::NotRead | TranscriptEvidence::Missing => {}
    }
    for id in ids {
        if let Some(owner) = db.get_session_binding(&id).map_err(internal)? {
            owners.insert(owner);
        }
    }
    Ok(match owners.len() {
        0 => Lineage::Unknown,
        1 => Lineage::Owner(owners.into_iter().next().unwrap()),
        _ => Lineage::Ambiguous,
    })
}

pub(super) fn observe(db: &AgentxDb, context: &RequestContext, input: Observation) -> ServiceResult<Resolution> {
    let effective_session = session(db, context, &input)?;
    // Revocation is an independently committed security action. Failure to
    // route a later status/lineage observation cannot roll a grant back alive.
    if let Some(tool_use) = input.terminal_shell_tool_use.as_deref().filter(|id| !id.is_empty()) {
        transaction(db, || db.revoke_claude_actor_capability(&effective_session, tool_use, input.agent_id.as_deref()).map_err(internal))?;
    }
    transaction(db, || {
        let result = crate::agents::service_store::retained(db, context, &json!({"claude_observe":input}), || observe_in_scope(db, context, &input, &effective_session))?;
        match &result {
            Resolution::Ready(view) => {
                let root = current(db, &view.actor.root)?;
                let child = current(db, &view.actor.participant)?;
                if root.session_id.as_deref() != Some(view.actor.session_id.as_str())
                    || view.actor.agent_id.as_deref().is_some_and(|id| child.agent_id.as_deref() != Some(id) || child.parent_name.as_deref() != Some(root.name.as_str()) || child.parent_session_id.as_deref() != Some(view.actor.session_id.as_str()))
                {
                    return Err(ServiceError::StaleFence);
                }
            }
            Resolution::Historical { root, .. } => {
                current(db, root)?;
            }
            _ => {}
        }
        Ok(result)
    })
}

fn observe_in_scope(db: &AgentxDb, context: &RequestContext, input: &Observation, session: &str) -> ServiceResult<Resolution> {
    let unbound = || Resolution::Unbound { session_id: session.to_owned() };
    let Some(process) = context.caller.invocation.process_binding_id.as_deref().filter(|value| !value.is_empty()) else {
        return Ok(unbound());
    };
    if session.is_empty() {
        return Ok(unbound());
    }
    let binding = db.get_process_binding_full(process).map_err(internal)?;
    let process_owner = binding.as_ref().map(|(_, name)| name.as_str());
    if let Some(name) = process_owner
        && db.get_instance_full(name).map_err(internal)?.is_some_and(|row| row.tool != "claude" || row.origin_device_id.is_some())
    {
        return Err(ServiceError::StaleFence);
    }
    let session_owner = db.get_session_binding(session).map_err(internal)?;
    let validated = db.get_validated_claude_session_owner(session).map_err(internal)?;
    let start = matches!(input.kind, ObservationKind::SessionStart { .. });
    if start && input.agent_id.is_some() {
        return Err(ServiceError::InvalidInput("Claude SessionStart must identify a root".into()));
    }
    let native = matches!(input.kind, ObservationKind::SessionStart { source: StartSource::Startup | StartSource::Fork | StartSource::Resume });
    let fresh = fresh(db, context, binding.as_ref(), Some(session), native)?;
    let fork = context.caller.invocation.environment.get("AGENTX_IS_FORK").map(String::as_str) == Some("1") && fresh;
    let compact = matches!(input.kind, ObservationKind::SessionStart { source: StartSource::Compact })
        && process_owner.is_some()
        && session_owner.as_deref() == process_owner
        && db.get_instance_full(process_owner.unwrap()).map_err(internal)?.is_some_and(|row| row.session_id.as_deref() == Some(session));
    let scan = if start { !compact && !fork && validated.is_none() && (native || session_owner.is_none() || session_owner.as_deref() != process_owner) } else { session_owner.is_none() || validated.as_ref() != session_owner.as_ref() };
    if scan && matches!(input.evidence, TranscriptEvidence::NotRead) {
        return Ok(Resolution::NeedsLineage { session_id: session.to_owned() });
    }
    let evidence = if scan { lineage(db, input, session, validated.is_some())? } else { Lineage::Unknown };
    if matches!(evidence, Lineage::Ambiguous) {
        return Ok(Resolution::Ambiguous { session_id: session.to_owned() });
    }
    if start && fork && session_owner.as_deref().is_some_and(|owner| Some(owner) != process_owner) {
        return Ok(unbound());
    }
    let selected = validated.clone().or_else(|| match &evidence {
        Lineage::Owner(owner) => Some(owner.clone()),
        _ => None,
    });
    let name = if start {
        if native && !fresh && scan && selected.is_none() {
            return Ok(unbound());
        }
        if let Some(selected) = selected {
            let row = db.get_instance_full(&selected).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
            current(db, &participant(&row))?;
            root_claims(db, context, &row)?;
            db.attach_claude_generation(&selected, session, input.transcript_path.as_deref().unwrap_or(""), process, process_owner).map_err(internal)?;
            selected
        } else if compact {
            process_owner.unwrap().to_owned()
        } else {
            if process_owner.is_none() && context.caller.invocation.environment.get("AGENTX_LAUNCHED").map(String::as_str) != Some("1") {
                return Ok(unbound());
            }
            let mut effective = context.clone();
            if let Some(directory) = &input.directory {
                effective.caller.invocation.cwd = directory.clone();
            }
            let Some(bound) = crate::agents::invocation_binding::bind_session_with_creation(db, &effective, session, "claude", true)? else {
                return Ok(unbound());
            };
            db.attach_claude_generation(&bound.record.name, session, input.transcript_path.as_deref().unwrap_or(""), process, Some(&bound.record.name)).map_err(internal)?;
            bound.record.name
        }
    } else {
        let selected = if let Some(selected) = selected {
            Some(selected)
        } else if scan {
            let historical = binding.as_ref().and_then(|(session, _)| session.as_deref()).is_some_and(|bound| !bound.is_empty() && bound != session);
            if session_owner.is_some() || historical { None } else { process_owner.map(str::to_owned) }
        } else {
            session_owner.clone().or_else(|| process_owner.map(str::to_owned))
        };
        let Some(selected) = selected else {
            return Ok(unbound());
        };
        if process_owner != Some(selected.as_str()) {
            return Ok(unbound());
        }
        selected
    };
    let mut root_record = db.get_instance_full(&name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
    current(db, &participant(&root_record))?;
    root_claims(db, context, &root_record)?;
    if root_record.parent_name.as_deref().is_some_and(|parent| !parent.is_empty()) {
        return Err(ServiceError::StaleFence);
    }
    if root_record.session_id.as_deref() != Some(session) {
        return Ok(Resolution::Historical { session_id: session.to_owned(), root: participant(&root_record) });
    }
    if !start && validated.is_none() {
        if !matches!(evidence,Lineage::Owner(ref owner) if owner==&name) || session_owner.as_deref() != Some(name.as_str()) {
            return Ok(unbound());
        }
        db.mark_claude_session_validated(session, &name).map_err(internal)?;
    }
    if start {
        crate::agents::invocation_binding::capture_launch_context(db, context, &name)?;
        if !matches!(input.kind, ObservationKind::SessionStart { source: StartSource::Compact }) {
            status(db, &root_record, "listening", "start", "", "", "")?;
        }
    }
    let root_fence = participant(&root_record);
    let (record, agent_id) = if let Some(id) = input.agent_id.as_deref().filter(|id| !id.is_empty()) {
        let existing = db.get_instance_by_agent_id(id).map_err(internal)?.map(|name| db.get_instance_full(&name).map_err(internal)?.ok_or(ServiceError::StaleFence)).transpose()?;
        if existing.as_ref().is_some_and(|row| row.parent_name.as_deref() != Some(name.as_str()) || row.parent_session_id.as_deref() != Some(session) || row.tool != "claude" || row.origin_device_id.is_some()) {
            return Err(ServiceError::StaleFence);
        }
        let child = if let ObservationKind::SubagentStart { agent_type } = &input.kind {
            let allocation = crate::agents::names::SubagentAllocation { agent_id: id, agent_type, parent_name: &name, parent_session_id: Some(session), status: "inactive", status_context: Some("subagent:dormant") };
            let directory = input.directory.as_ref().unwrap_or(&context.caller.invocation.cwd).to_string_lossy();
            let child = crate::agents::names::allocate_subagent_instance_in_directory(db, &allocation, &directory).map_err(internal)?;
            db.get_instance_full(&child).map_err(internal)?.ok_or(ServiceError::StaleFence)?
        } else {
            let Some(child) = existing else {
                return Ok(unbound());
            };
            child
        };
        db.update_instance_fields(&child.name, json!({"last_seen":crate::shared::time::now_epoch_i64()}).as_object().unwrap()).map_err(internal)?;
        (db.get_instance_full(&child.name).map_err(internal)?.ok_or(ServiceError::StaleFence)?, Some(id.to_owned()))
    } else {
        if matches!(input.kind, ObservationKind::SubagentStart { .. }) {
            return Err(ServiceError::InvalidInput("SubagentStart agent id required".into()));
        }
        let mut updates = serde_json::Map::new();
        if let Some(path) = &input.transcript_path
            && !path.is_empty()
        {
            updates.insert("transcript_path".into(), json!(path));
        }
        if let Some(directory) = &input.directory {
            updates.insert("directory".into(), json!(directory.to_string_lossy()));
        }
        if !updates.is_empty() {
            db.update_instance_fields(&name, &updates).map_err(internal)?;
        }
        root_record = db.get_instance_full(&name).map_err(internal)?.ok_or(ServiceError::StaleFence)?;
        (root_record.clone(), None)
    };
    let actor = Actor { root: root_fence, participant: participant(&record), session_id: session.to_owned(), agent_id };
    let bootstrap = if start { Some(super::output::bootstrap(db, context, &actor, true)?) } else { None };
    Ok(Resolution::Ready(ActorView { actor, record, root_record, bootstrap }))
}
