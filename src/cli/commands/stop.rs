//! `agentx stop` command — end agentx participation.
//!
//!
//! Supports: self-stop, named stop, multi-stop and `all`.
//! Inside AI tools, destructive ops require `--go` flag.

#[cfg(test)]
use crate::agents::identity;
#[cfg(test)]
use crate::agents::records::{is_remote_instance, is_subagent_instance};
#[cfg(test)]
use crate::db::AgentxDb;
#[cfg(test)]
use crate::integrations::hooks::common::stop_instance;
#[cfg(test)]
use crate::log::log_info;
#[cfg(test)]
use crate::shared::{CommandContext, SENDER, SenderKind, is_inside_ai_tool};

/// Parsed arguments for `agentx stop`.
#[derive(clap::Parser, Debug)]
#[command(name = "stop", about = "Stop agentx participation")]
pub struct StopArgs {
    /// Targets to stop (names or "all")
    pub targets: Vec<String>,
}

/// Resolve the initiator name for event logging.
#[cfg(test)]
fn resolve_initiator(db: &AgentxDb, ctx: Option<&CommandContext>, explicit_name: Option<&str>) -> String {
    if let Some(c) = ctx
        && let Some(ref id) = c.identity
        && matches!(id.kind, SenderKind::Instance)
    {
        return id.name.clone();
    }
    if let Some(name) = explicit_name {
        return name.to_string();
    }
    match identity::resolve_identity(db, None, None, None, None, None) {
        Ok(id) => id.name,
        Err(_) => "cli".to_string(),
    }
}

/// Main entry point for `agentx stop` command.
///
/// Returns exit code (0 = success, 1 = error).
#[cfg(test)]
pub fn cmd_stop(db: &AgentxDb, args: &StopArgs, ctx: Option<&CommandContext>) -> i32 {
    let explicit_name = ctx.and_then(|c| c.explicit_name.as_deref());

    // Give rejoined PTYs back their pid first, so stopping one tracks its
    // process under the name it runs as now.
    crate::runtime::process_tracking::claim_orphans(db, &crate::paths::agentx_dir());

    let targets: Vec<&str> = args.targets.iter().map(|s| s.as_str()).collect();

    // Handle 'all' target
    if targets.contains(&"all") {
        if targets.len() > 1 {
            eprintln!("Error: 'all' cannot be combined with other targets");
            return 1;
        }

        // Only stop local instances
        let instances = match db.iter_instances_full() {
            Ok(rows) => rows.into_iter().filter(|i| !is_remote_instance(i)).collect::<Vec<_>>(),
            Err(e) => {
                eprintln!("Error: {e}");
                return 1;
            }
        };

        if instances.is_empty() {
            println!("Nothing to stop");
            return 0;
        }

        // Confirmation gate: inside AI tools, require --go
        if is_inside_ai_tool() && !ctx.map(|c| c.go).unwrap_or(false) {
            print_stop_preview(db, "ALL", "all", &instances);
            return 0;
        }

        let launcher = resolve_initiator(db, ctx, explicit_name);
        log_info("lifecycle", "stop.all", &format!("count={} initiated_by={launcher}", instances.len()));

        let mut stopped_names = Vec::new();
        let mut bg_logs = Vec::new();
        let mut notes = Vec::new();

        for inst in &instances {
            let display = inst.name.clone();
            notes.extend(stop_one(db, &inst.name, &launcher, "stop_all"));
            stopped_names.push(display.clone());

            if inst.background != 0 && !inst.background_log_file.is_empty() {
                bg_logs.push((display, inst.background_log_file.clone()));
            }
        }

        if stopped_names.is_empty() {
            println!("Nothing to stop");
        } else {
            println!("Stopped: {}", stopped_names.join(", "));
            print_notes(&notes);
            if !bg_logs.is_empty() {
                println!("\nHeadless logs:");
                for (name, log_file) in &bg_logs {
                    println!("  {name}: {log_file}");
                }
            }
        }
        return 0;
    }

    // Handle multiple explicit targets
    if targets.len() > 1 {
        let mut instances_to_stop = Vec::new();
        let mut not_found = Vec::new();
        let mut already_stopped = Vec::new();

        for t in &targets {
            let name = t;
            match db.get_instance_full(name) {
                Ok(Some(data)) => instances_to_stop.push(data),
                _ => match identity::last_stopped(db, t) {
                    Some(stopped) => already_stopped.push((t.to_string(), stopped)),
                    None => not_found.push(t.to_string()),
                },
            }
        }

        if !not_found.is_empty() {
            for t in &not_found {
                eprintln!("Error: {}", identity::describe_missing_agent(db, t));
            }
            return 1;
        }
        for (t, stopped) in &already_stopped {
            print_already_stopped(t, stopped);
        }
        if instances_to_stop.is_empty() {
            return 0;
        }

        // Confirmation gate
        if is_inside_ai_tool() && !ctx.map(|c| c.go).unwrap_or(false) {
            print_stop_preview(db, "", &targets.join(" "), &instances_to_stop);
            return 0;
        }

        let launcher = resolve_initiator(db, ctx, explicit_name);
        let mut stopped_names = Vec::new();
        let mut bg_logs = Vec::new();
        let mut notes = Vec::new();

        for inst in &instances_to_stop {
            if is_remote_instance(inst) {
                println!("Skipping remote instance: {}", inst.name);
                continue;
            }
            let display = inst.name.clone();
            notes.extend(stop_one(db, &inst.name, &launcher, "multi_stop"));
            stopped_names.push(display.clone());
            if inst.background != 0 && !inst.background_log_file.is_empty() {
                bg_logs.push((display, inst.background_log_file.clone()));
            }
        }

        if !stopped_names.is_empty() {
            println!("Stopped: {}", stopped_names.join(", "));
        }
        print_notes(&notes);
        if !bg_logs.is_empty() {
            println!("\nHeadless logs:");
            for (name, log_file) in &bg_logs {
                println!("  {name}: {log_file}");
            }
        }
        return 0;
    }

    // Single target or self-stop
    let instance_name = if !targets.is_empty() {
        // Named target
        let target = targets[0];
        target.to_string()
    } else {
        // Self-stop: resolve identity
        let identity =
            if let Some(c) = ctx { if let Some(ref id) = c.identity { Some(id.clone()) } else { identity::resolve_identity(db, explicit_name, None, None, None, None).ok() } } else { identity::resolve_identity(db, None, None, None, None, None).ok() };

        match identity {
            Some(id) => id.name,
            None => {
                eprintln!("Error: Cannot determine identity\nUsage: agentx stop <name> | agentx stop all | run 'agentx stop' inside an agentx agent");
                return 1;
            }
        }
    };

    // Handle SENDER (not real instance)
    if instance_name == SENDER {
        eprintln!("Error: Cannot resolve identity - launch via 'agentx <n>' for stable identity");
        return 1;
    }

    // Lookup instance
    let position = match db.get_instance_full(&instance_name) {
        Ok(Some(data)) => data,
        _ => {
            if let Some(stopped) = identity::last_stopped(db, &instance_name) {
                print_already_stopped(&instance_name, &stopped);
                return 0;
            }
            eprintln!("Error: {}", identity::describe_missing_agent(db, &instance_name));
            return 1;
        }
    };

    // Remote instances are mirrors only. Stopping them remotely would strand the
    // agent from agentx without giving a useful way to recover/control it remotely.
    if is_remote_instance(&position) {
        eprintln!("Error: Remote stop is not supported for '{instance_name}'. Use remote kill or ask the agent to stop itself locally.");
        return 1;
    }

    let launcher = resolve_initiator(db, ctx, explicit_name);
    let is_external_stop = !targets.is_empty();
    let reason = if is_external_stop { "external" } else { "self" };

    let display = position.name.clone();
    log_info("lifecycle", "stop.single", &format!("name={instance_name} reason={reason} initiated_by={launcher}"));

    if let Some(note) = stop_one(db, &instance_name, &launcher, reason) {
        println!("{note}");
        return 0;
    }

    if is_subagent_instance(&position) {
        println!("Stopped agentx for subagent {display}.");
    } else {
        println!("Stopped agentx for {display}.");
    }

    if position.background != 0 && !position.background_log_file.is_empty() {
        println!("\nHeadless log: {}", position.background_log_file);
    }

    0
}

/// Stop one instance. A Hermes chat bridge is disconnected instead (an
/// instance stop alone would be undone by its gateway's next sync); returns
/// what that did, to print.
#[cfg(test)]
fn stop_one(db: &AgentxDb, name: &str, launcher: &str, reason: &str) -> Option<String> {
    let note = crate::integrations::hooks::hermes::bridge::disconnect(db, name, launcher);
    if note.is_none() {
        stop_instance(db, name, launcher, reason);
    }
    note
}

fn print_notes(notes: &[String]) {
    for note in notes {
        println!("\n{note}");
    }
}

/// Print a stop preview for a named or all-instance scope.
#[cfg(test)]
fn print_stop_preview(db: &AgentxDb, scope: &str, cmd_suffix: &str, instances: &[crate::db::InstanceRow]) {
    let count = instances.len();
    let names: Vec<String> = instances.iter().map(|instance| instance.name.clone()).collect();
    let bridge_names: std::collections::HashSet<String> = crate::integrations::hooks::hermes::bridge::bridges(db).into_iter().map(|b| b.name).collect();
    let bridged = instances.iter().filter(|i| bridge_names.contains(&i.name)).count();
    let headless = instances.iter().filter(|i| i.background != 0).count();
    let interactive = count - headless - bridged;
    let instance_list = if count <= 8 { names.join(", ") } else { format!("{} ... (+{} more)", names[..6].join(", "), count - 6) };

    println!("\n== STOP {scope} PREVIEW ==");
    println!("This will stop {count} instance{}.\n", if count != 1 { "s" } else { "" });
    println!("Instances to stop:\n  {instance_list}\n");
    println!("What happens:");
    println!("  • Headless instances ({headless}): process killed (SIGTERM, then SIGKILL after 2s)");
    println!("  • Interactive instances ({interactive}): notified via TCP (graceful)");
    if bridged > 0 {
        println!("  • Hermes chat bridges ({bridged}): disconnected, nothing killed; /agentx start <name> in the chat reconnects");
    }
    println!("  • All: stopped event logged with snapshot, instance rows deleted");
    println!("  • Subagents: recursively stopped when parent stops\n");
    println!("Instance data preserved in events table (life.stopped with snapshot).\n");
    println!("Add --go flag and run again to proceed:");
    println!("  agentx --go stop {cmd_suffix}\n");
}

/// A stop can leave the PTY process running (tracked as an orphan), so an
/// earlier stop event alone doesn't mean nothing is left to end.
#[cfg(test)]
fn print_already_stopped(target: &str, stopped: &identity::StoppedAgent) {
    println!("'{target}' {}", stopped.summary());
    // Match by session, not name: a reused base name must not attribute
    // another incarnation's process to the stop described above.
    if stopped.reason == crate::integrations::hooks::hermes::bridge::DISCONNECTED {
        println!("  {}", crate::integrations::hooks::hermes::bridge::reconnect_hint(&stopped.name));
        return;
    }
    let display = stopped.name.clone();
    match crate::cli::commands::kill::find_orphan_for_session(&stopped.session_id) {
        Some(orphan) => println!("  Its process is still running (pid {pid}). To end it: agentx kill {pid}", pid = orphan.pid),
        None => println!("  To resume: agentx r {display}"),
    }
}

use crate::agents::attachments::ParticipantRef;
use crate::agents::{
    hermes_service as hermes,
    hook_service::{OperatorStop, SessionStopOutcome},
};
use crate::cli::client::Session;
use crate::service::{Request, Response, ServiceError, ServiceResult};

pub(crate) fn participant(row: &crate::db::InstanceRow) -> ParticipantRef {
    ParticipantRef { canonical_name: row.name.clone(), incarnation_bits: row.created_at.to_bits() }
}
pub(crate) fn initiator(session: &Session) -> &str {
    session.actor.as_ref().map_or("cli", |actor| actor.record.name.as_str())
}
pub(crate) fn selected<'a>(records: &'a [crate::db::InstanceRow], target: &str) -> Option<&'a crate::db::InstanceRow> {
    records.iter().find(|row| row.name == target)
}
pub(crate) fn stopped(session: &Session, target: &str) -> ServiceResult<Option<crate::agents::identity::StoppedAgent>> {
    match session.client.call(Request::Queries(crate::service::queries::Request::StoppedTargets { targets: vec![target.into()] }))? {
        Response::Queries(crate::service::queries::Response::StoppedTargets(mut rows)) => Ok(rows.pop()),
        _ => Err(ServiceError::Internal("unexpected stopped participant result".into())),
    }
}
pub(crate) fn bridge_entries(session: &Session) -> ServiceResult<Vec<hermes::RegistryEntry>> {
    match session.client.call(Request::Participants(crate::agents::service::Request::Hermes(hermes::Request::List { home: None })))? {
        Response::Participants(crate::agents::service::Response::Hermes(hermes::Response::Registry(entries))) => Ok(entries),
        _ => Err(ServiceError::Internal("unexpected Hermes registry listing".into())),
    }
}
pub(crate) fn disconnect_bridge(session: &Session, row: &crate::db::InstanceRow) -> ServiceResult<Option<(String, SessionStopOutcome)>> {
    let fence = participant(row);
    let Some(expected) = bridge_entries(session)?.into_iter().find(|entry| entry.bridge.name == row.name) else {
        return Ok(None);
    };
    if expected.participant.as_ref() != Some(&fence) {
        return Err(ServiceError::StaleFence);
    }
    let caller = crate::service::CallerContext {
        request_id: crate::service::RequestId(uuid::Uuid::new_v4().to_string()),
        invocation: session.invocation.clone(),
        deadline_unix_ms: None,
        expected_database_generation: Some(session.client.info().database_generation.clone()),
    };
    let request = Request::Participants(crate::agents::service::Request::Hermes(hermes::Request::Remove { expected }));
    let response = match session.client.call_with_context(caller.clone(), request.clone()) {
        Err(ServiceError::OutcomeUnknown(_)) => session.client.call_with_context(caller, request),
        result => result,
    }?;
    let Response::Participants(crate::agents::service::Response::Hermes(hermes::Response::Removed { bridge, effects })) = response else {
        return Err(ServiceError::Internal("unexpected Hermes disconnect result".into()));
    };
    if session.actor.as_ref().is_some_and(|actor| effects.iter().any(|effect| effect.participant == actor.participant)) {
        crate::cli::context::claim_inline_delivery();
    }
    for effect in effects.iter().cloned() {
        crate::runtime::stop_effects::apply(effect);
    }
    let label = if bridge.chat.is_empty() { bridge.platform } else { format!("{} chat {}", bridge.platform, bridge.chat) };
    let note =
        format!("Disconnected '{}' ({label}) from agentx. Nothing was killed: the Hermes gateway and the chat keep running, other agents just can't message it.\n{}", row.name, crate::integrations::hooks::hermes::bridge::reconnect_hint(&row.name));
    Ok(Some((note, SessionStopOutcome { stopped: true, retained: false, effects })))
}
pub(crate) fn stop_one_client(session: &Session, row: &crate::db::InstanceRow, decision: &OperatorStop) -> ServiceResult<(Option<String>, SessionStopOutcome)> {
    if let Some((note, outcome)) = disconnect_bridge(session, row)? {
        return Ok((Some(note), outcome));
    }
    let outcome = crate::runtime::stop_effects::retire(&session.client, &participant(row), initiator(session), decision)?;
    if session.actor.as_ref().is_some_and(|actor| outcome.effects.iter().any(|effect| effect.participant == actor.participant)) {
        crate::cli::context::claim_inline_delivery();
    }
    Ok((None, outcome))
}

pub(crate) fn cmd_stop_client(session: &Session, args: &StopArgs) -> i32 {
    match stop_client(session, args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error}");
            1
        }
    }
}
fn stop_client(session: &Session, args: &StopArgs) -> ServiceResult<i32> {
    crate::runtime::stop_effects::claim_orphans(&session.client)?;
    let records = session.records()?;
    let all = args.targets.iter().any(|target| target == "all");
    if all && args.targets.len() > 1 {
        return Err(ServiceError::InvalidInput("'all' cannot be combined with other targets".into()));
    }
    let mut rows = Vec::new();
    if all {
        rows = records.iter().filter(|row| !crate::agents::records::is_remote_instance(row)).cloned().collect();
        if rows.is_empty() {
            println!("Nothing to stop");
            return Ok(0);
        }
    } else {
        let targets = if args.targets.is_empty() {
            vec![session.actor.as_ref().ok_or_else(|| ServiceError::IdentityRequired("Cannot determine identity\nUsage: agentx stop <name> | agentx stop all | run 'agentx stop' inside an agentx agent".into()))?.record.name.clone()]
        } else {
            args.targets.clone()
        };
        for target in &targets {
            if let Some(row) = selected(&records, target) {
                rows.push(row.clone());
            } else if let Some(prior) = stopped(session, target)? {
                print_already_stopped_client(session, target, &prior);
            } else {
                return Err(ServiceError::NotFound(format!("Agent '{target}' not found")));
            }
        }
        if rows.len() == 1 && crate::agents::records::is_remote_instance(&rows[0]) {
            return Err(ServiceError::InvalidInput(format!("Remote stop is not supported for '{}'. Use remote kill or ask the agent to stop itself locally.", rows[0].name)));
        }
    }
    let runtime = crate::runtime::context::AgentxContext::from_env(&session.invocation.environment, session.invocation.cwd.clone());
    if (all || args.targets.len() > 1) && runtime.is_inside_ai_tool() && !session.command.go {
        print_stop_preview_client(session, &args.targets.join(" "), &rows)?;
        return Ok(0);
    }
    let reason = if all {
        "stop_all"
    } else if args.targets.len() > 1 {
        "multi_stop"
    } else if args.targets.is_empty() {
        "self"
    } else {
        "external"
    };
    let (mut names, mut notes, mut logs) = (Vec::new(), Vec::new(), Vec::new());
    let mut retired = std::collections::HashSet::new();
    for row in &rows {
        if crate::agents::records::is_remote_instance(row) {
            println!("Skipping remote instance: {}", row.name);
            continue;
        }
        if retired.contains(&participant(row)) {
            continue;
        }
        let (note, outcome) = stop_one_client(session, row, &OperatorStop::Participation { reason: reason.into() })?;
        retired.extend(outcome.effects.into_iter().map(|effect| effect.participant));
        let display = row.name.clone();
        names.push(display.clone());
        if let Some(note) = note {
            notes.push(note);
        }
        if row.background != 0 && !row.background_log_file.is_empty() {
            logs.push((display, row.background_log_file.clone()));
        }
    }
    if all || args.targets.len() > 1 {
        if !names.is_empty() {
            println!("Stopped: {}", names.join(", "));
        }
        print_notes(&notes);
        if !logs.is_empty() {
            println!("\nHeadless logs:");
            for (name, path) in logs {
                println!("  {name}: {path}");
            }
        }
    } else if let Some(row) = rows.first() {
        if notes.is_empty() {
            println!("Stopped agentx for {}{}.", if crate::agents::records::is_subagent_instance(row) { "subagent " } else { "" }, row.name);
        } else {
            for note in notes {
                println!("{note}");
            }
        }
        if let Some((_, path)) = logs.first() {
            println!("\nHeadless log: {path}");
        }
    }
    Ok(0)
}
fn print_stop_preview_client(session: &Session, suffix: &str, rows: &[crate::db::InstanceRow]) -> ServiceResult<()> {
    let bridge_names: std::collections::HashSet<_> = bridge_entries(session)?.into_iter().map(|entry| entry.bridge.name).collect();
    let bridged = rows.iter().filter(|row| bridge_names.contains(&row.name)).count();
    let headless = rows.iter().filter(|row| row.background != 0).count();
    let names: Vec<_> = rows.iter().map(|row| row.name.clone()).collect();
    println!(
        "\n== STOP {suffix} PREVIEW ==\nThis will stop {} instances.\n\nInstances to stop:\n  {}\n\nWhat happens:\n  • Headless instances ({headless}): process killed (SIGTERM, then SIGKILL after 2s)\n  • Interactive instances ({}): notified via TCP (graceful)\n  • Hermes chat bridges ({bridged}): disconnected; /agentx start <name> in the chat reconnects\n  • All: stopped event logged with snapshot, instance rows deleted\n  • Subagents: recursively stopped when parent stops\n\nAdd --go flag and run again to proceed:\n  agentx --go stop {suffix}\n",
        rows.len(),
        names.join(", "),
        rows.len().saturating_sub(headless + bridged)
    );
    Ok(())
}
fn print_already_stopped_client(session: &Session, target: &str, prior: &crate::agents::identity::StoppedAgent) {
    println!("'{target}' {}", prior.summary());
    if prior.reason == crate::integrations::hooks::hermes::bridge::DISCONNECTED {
        println!("  {}", crate::integrations::hooks::hermes::bridge::reconnect_hint(&prior.name));
        return;
    }
    let orphan = crate::runtime::process_tracking::get_orphan_processes(session.client.scope_root(), None).into_iter().find(|orphan| !prior.session_id.is_empty() && orphan.session_id == prior.session_id);
    if let Some(orphan) = orphan {
        println!("  Its process is still running (pid {pid}). To end it: agentx kill {pid}", pid = orphan.pid);
    } else {
        println!("  To resume: agentx r {}", prior.name);
    }
}
