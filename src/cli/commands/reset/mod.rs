//! `agentx reset` command — archive and clear conversation, optionally hooks/config.
//!
//!
//! Modes:
//!   agentx reset              Clear database (archive conversation)
//!   agentx reset hooks        Remove hooks
//!   agentx reset all          Stop all + clear db + remove hooks + reset config

pub(crate) mod operations;
pub(crate) mod preview;

use crate::cli::client::Session;
#[cfg(test)]
use crate::db::AgentxDb;
use crate::service::{Request, Response, ServiceError, ServiceResult, maintenance as M};
#[cfg(test)]
use crate::shared::{CommandContext, is_inside_ai_tool};

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetTarget {
    Hooks,
    All,
}

#[derive(clap::Parser, Debug)]
#[command(name = "reset", about = "Reset agentx components")]
pub struct ResetArgs {
    /// Reset scope
    #[arg(value_enum)]
    pub target: Option<ResetTarget>,
}

/// Hook removal is host-only, including its AI confirmation preview.
pub(crate) fn try_local(args: &ResetArgs, go: bool) -> Option<i32> {
    if args.target != Some(ResetTarget::Hooks) {
        return None;
    }
    let invocation = match crate::service::InvocationContext::capture() {
        Ok(invocation) => invocation,
        Err(error) => {
            eprintln!("Error: {error}");
            return Some(1);
        }
    };
    Some(hooks_for_invocation(&invocation, go))
}

fn needs_preview(invocation: &crate::service::InvocationContext, go: bool) -> bool {
    !go && crate::runtime::context::AgentxContext::from_env(&invocation.environment, invocation.cwd.clone()).is_inside_ai_tool()
}

fn hooks_for_invocation(invocation: &crate::service::InvocationContext, go: bool) -> i32 {
    if needs_preview(invocation, go) {
        preview::print_hooks_preview();
        0
    } else {
        crate::cli::commands::hooks::cmd_hooks_remove(&["all".into()])
    }
}

/// Retain the originating Client through stop/worker shutdown/reset. Only a
/// confirmed reset permits a deliberate binding to its returned generation.
pub(crate) fn cmd_reset_client(session: &Session, args: &ResetArgs) -> i32 {
    match reset_client(session, args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error}");
            1
        }
    }
}

fn summary(client: &crate::service::client::Client) -> ServiceResult<M::DatabaseSummary> {
    match client.call(Request::Maintenance(M::Request::Summary))? {
        Response::Maintenance(M::Response::Summary(summary)) => Ok(summary),
        _ => Err(ServiceError::Internal("daemon returned a different reset summary".into())),
    }
}

/// Reset has no retained cross-generation recovery contract. Never retry an
/// unknown reply or infer that it committed by observing a new generation.
fn commit_reset(client: &crate::service::client::Client) -> ServiceResult<M::ResetResult> {
    match client.call(Request::Maintenance(M::Request::ResetConversation))? {
        Response::Maintenance(M::Response::Reset(result)) => Ok(result),
        _ => Err(ServiceError::OutcomeUnknown("unexpected reset response; inspect 'agentx archive' and 'agentx status' before issuing another reset".into())),
    }
}

fn notify_reset(client: &crate::service::client::Client, result: &M::ResetResult) -> ServiceResult<()> {
    let next = crate::service::client::Client::connect_existing_fenced(client.scope_root(), client.invocation().clone(), &result.database_generation)?;
    let config = crate::config::AgentxConfig::load_for_scope(next.scope_root(), &next.invocation().environment);
    if crate::relay::is_relay_enabled(&config) {
        crate::relay::trigger_push_client(&next);
    }
    Ok(())
}

fn stop_relay(client: &crate::service::client::Client) -> ServiceResult<()> {
    crate::relay::worker::stop_relay_worker_client(client).map_err(ServiceError::Unavailable)
}

fn reset_client(session: &Session, args: &ResetArgs) -> ServiceResult<i32> {
    if args.target == Some(ResetTarget::Hooks) {
        return Ok(hooks_for_invocation(&session.invocation, session.command.go));
    }
    let state = summary(&session.client)?;
    if needs_preview(&session.invocation, session.command.go) {
        preview::print_reset_preview_client(args.target, &state, session.client.scope_root());
        return Ok(0);
    }
    // Stop may retire the command actor before reset. No mailbox append can
    // use that actor or the old generation, even if a later step fails.
    crate::cli::context::claim_inline_delivery();
    let stop = crate::cli::commands::stop::StopArgs { targets: vec!["all".into()] };
    if crate::cli::commands::stop::cmd_stop_client(session, &stop) != 0 {
        return Ok(1);
    }
    stop_relay(&session.client)?;
    let result = commit_reset(&session.client).map_err(|error| match error {
        ServiceError::OutcomeUnknown(detail) => {
            ServiceError::OutcomeUnknown(format!("reset reply was lost ({detail}); inspect 'agentx archive' and 'agentx status' before issuing another reset. Configuration and full-reset files were left in place"))
        }
        other => other,
    })?;
    operations::print_archive_result(Ok(result.archive.clone()));
    let mut code = match notify_reset(&session.client, &result) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("Reset committed; relay notification unavailable: {error}");
            1
        }
    };
    let scope = session.client.scope_root();
    operations::clean_temp_files_for_scope(scope);
    if args.target == Some(ResetTarget::All) {
        if let Err(error) = operations::clear_full_reset_artifacts_for_scope(scope) {
            eprintln!("Conversation reset committed; file cleanup failed: {error}");
            code = 1;
        }
        code = code.max(crate::cli::commands::hooks::cmd_hooks_remove(&["all".into()]));
        code = code.max(operations::reset_config_for_scope(scope));
    }
    Ok(code)
}

#[cfg(test)]
pub fn cmd_reset(db: AgentxDb, args: &ResetArgs, ctx: Option<&CommandContext>) -> i32 {
    if let Some(exit_code) = try_cmd_reset_preserving_db(&db, args, ctx) {
        return exit_code;
    }

    cmd_reset_database(db, args, ctx)
}

/// Handle reset modes that do not replace the database.
///
/// The router calls this first so it can retain its connection and perform
/// normal post-command delivery. `cmd_reset` also calls it for direct callers.
#[cfg(test)]
pub(crate) fn try_cmd_reset_preserving_db(db: &AgentxDb, args: &ResetArgs, ctx: Option<&CommandContext>) -> Option<i32> {
    let target = args.target;

    // Confirmation gate: inside AI tools, require --go
    if is_inside_ai_tool() && !ctx.map(|c| c.go).unwrap_or(false) {
        crate::cli::commands::reset::preview::print_reset_preview(target, db);
        return Some(0);
    }

    // hooks: remove hooks from all locations
    if target == Some(ResetTarget::Hooks) {
        return Some(crate::cli::commands::hooks::cmd_hooks_remove(&["all".to_string()]));
    }

    None
}

#[cfg(test)]
fn cmd_reset_database(db: AgentxDb, args: &ResetArgs, ctx: Option<&CommandContext>) -> i32 {
    let target = args.target;
    let mut exit_codes = Vec::new();

    // Stop all instances before clearing database
    let stop_args = crate::cli::commands::stop::StopArgs { targets: vec!["all".into()] };
    exit_codes.push(crate::cli::commands::stop::cmd_stop(&db, &stop_args, ctx));

    // Stop relay daemon if running before clear
    let _ = crate::cli::commands::daemon::daemon_stop();

    // Clean temp files
    crate::cli::commands::reset::operations::clean_temp_files();

    // Explicitly release command-owned database connection before archive/clear.
    // On Windows, SQLite's open file handle prevents deleting agentx.db, agentx.db-wal, and agentx.db-shm.
    drop(db);

    // Archive and clear database
    let archive_exit = crate::cli::commands::reset::operations::print_archive_result(crate::cli::commands::reset::operations::archive_and_clear_db());
    if archive_exit != 0 {
        return archive_exit;
    }

    // For reset all: clear pidtrack before recovery can trigger
    if target == Some(ResetTarget::All) {
        crate::cli::commands::reset::operations::clear_full_reset_artifacts();
    }

    // Log reset event to fresh DB
    crate::cli::commands::reset::operations::bootstrap_fresh_db();

    // Respawn relay worker (was stopped above) and push reset event to remote devices.
    // ensure_worker re-reads config, so this is a no-op when relay is not configured.
    crate::relay::worker::ensure_worker();
    crate::relay::trigger_push();

    // all: also remove hooks, reset config, clear device identity
    if target == Some(ResetTarget::All) {
        // Remove hooks
        if crate::cli::commands::hooks::cmd_hooks_remove(&["all".to_string()]) != 0 {
            exit_codes.push(1);
        }

        // Reset config
        exit_codes.push(crate::cli::commands::reset::operations::reset_config());
    }

    exit_codes.into_iter().max().unwrap_or(0)
}

#[cfg(test)]
mod client_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn reset_args_default_to_db_reset() {
        let args = ResetArgs::try_parse_from(["reset"]).unwrap();
        assert_eq!(args.target, None);
    }

    #[test]
    fn reset_args_parse_named_target() {
        let args = ResetArgs::try_parse_from(["reset", "hooks"]).unwrap();
        assert_eq!(args.target, Some(ResetTarget::Hooks));
    }
}
