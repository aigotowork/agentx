//! Originating listen host. Waiting and stdout stay here; the native Events
//! lifetime owns exact status/heartbeat/subscription/endpoint persistence.

use super::listen::{ListenArgs, expand_sql_preset, message_json};
use crate::cli::{client::Session, output::Batch};
use crate::events::service::{ListenEnd, ListenSpec, Request as Events, Response as EventResult};
use crate::runtime::notify::NotifyServer;
use crate::service::{CallerContext, Request, RequestId, Response, ServiceError, ServiceResult};
use std::io::Write;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

fn call(session: &Session, request: Events) -> ServiceResult<EventResult> {
    match session.client.call(Request::Events(request))? {
        Response::Events(result) => Ok(result),
        _ => Err(ServiceError::Internal("daemon returned a different listener response".into())),
    }
}

/// Begin/End are retained by listener ID, so a lost reply can recover using the
/// original request identity without reacquiring another daemon/generation.
fn retained_call(session: &Session, request: Events) -> ServiceResult<EventResult> {
    let caller = CallerContext { request_id: RequestId(uuid::Uuid::new_v4().to_string()), invocation: session.invocation.clone(), deadline_unix_ms: None, expected_database_generation: None };
    let request = Request::Events(request);
    let result = match session.client.call_with_context(caller.clone(), request.clone()) {
        Err(ServiceError::OutcomeUnknown(_)) => session.client.call_with_context(caller, request),
        result => result,
    }?;
    match result {
        Response::Events(result) => Ok(result),
        _ => Err(ServiceError::Internal("daemon returned a different listener response".into())),
    }
}

pub(crate) fn cmd_listen(session: &Session, args: &ListenArgs) -> i32 {
    match run(session, args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("agentx: {error}");
            1
        }
    }
}

fn run(session: &Session, args: &ListenArgs) -> ServiceResult<i32> {
    let actor = session.actor.as_ref().ok_or_else(|| ServiceError::IdentityRequired("no agentx identity. Run 'agentx start' to connect, or use 'agentx --iam <name> listen' for an existing participant".into()))?;
    let requested = args.timeout.or(args.timeout_positional).unwrap_or(86400);
    let timeout = if requested <= 1 { 0.1 } else { requested as f64 };
    let notify = NotifyServer::new().ok();
    let spec = ListenSpec {
        participant: actor.participant.clone(),
        id: format!("listen-{}", uuid::Uuid::new_v4()),
        port: notify.as_ref().map(NotifyServer::port),
        filters: args.filters.to_filter_map(),
        predicate: args.sql.as_deref().map(expand_sql_preset).transpose().map_err(|error| ServiceError::InvalidInput(error.into()))?,
    };
    let end = |reason| Events::EndListen { participant: spec.participant.clone(), id: spec.id.clone(), reason };
    let ready = match retained_call(session, Events::BeginListen(spec.clone())) {
        Ok(EventResult::ListenReady(ready)) => ready,
        Ok(_) => return Err(ServiceError::Internal("daemon returned a different listener setup".into())),
        Err(error) => {
            let _ = retained_call(session, end(None));
            return Err(error);
        }
    };
    if let Some(event) = ready.recent {
        let output = if args.json {
            format!("{}\n", serde_json::json!({"event_id":event["id"],"type":event["type"],"instance":event["instance"],"data":event["data"]}))
        } else {
            format!("[Match found] #{} {}:{}\n", event["id"], event["type"].as_str().unwrap_or(""), event["instance"].as_str().unwrap_or(""))
        };
        write(&output)?;
        return Ok(0);
    }
    let shutdown = Arc::new(AtomicBool::new(false));
    crate::sys::signal::register_term(&shutdown);
    crate::sys::signal::register_int(&shutdown);
    if !args.json {
        if ready.filtered {
            eprintln!("[Listening for events matching filter. Timeout: {timeout}s]");
        } else {
            eprintln!("[Listening for messages to {}. Timeout: {timeout}s]", actor.record.name);
        }
    }
    let result = wait(session, args, &spec, timeout, notify.as_ref(), &shutdown);
    let reason = result.as_ref().ok().and_then(|(_, reason)| reason.clone());
    let cleanup = retained_call(session, end(reason));
    // Preserve the first failure; cleanup still runs after interruption/output
    // errors, and can only delete this exact lifetime's durable resources.
    let (code, _) = result?;
    match cleanup? {
        EventResult::ListenEnded => Ok(code),
        _ => Err(ServiceError::Internal("daemon returned a different listener cleanup".into())),
    }
}

fn write(output: &str) -> ServiceResult<()> {
    let mut out = std::io::stdout().lock();
    out.write_all(output.as_bytes()).and_then(|()| out.flush()).map_err(|error| ServiceError::OutcomeUnknown(format!("listen output failed: {error}")))
}

fn notes(session: &Session, args: &ListenArgs, spec: &ListenSpec, batch: &Batch) -> ServiceResult<()> {
    let mut note = batch.remaining_note(&spec.participant.canonical_name);
    if batch.arrived_since(&session.client, &spec.participant)? {
        note.push_str(&format!("[agentx] new message(s) arrived — run: agentx listen --iam {}\n", spec.participant.canonical_name));
    }
    if args.json {
        eprint!("{note}");
        Ok(())
    } else {
        write(&note)
    }
}

fn batch_output(session: &Session, args: &ListenArgs, spec: &ListenSpec, batch: &Batch, filtered: bool) -> ServiceResult<(String, bool, bool)> {
    let tag = format!("[sub:{}]", spec.id);
    let is_match = |message: &crate::db::Message| filtered && message.from == "[agentx-events]" && message.text.contains(&tag);
    let matched = batch.messages.iter().any(is_match);
    let human = batch.messages.iter().any(|message| !message.from.starts_with('['));
    let records = if args.json { Vec::new() } else { session.records()? };
    let mut output = String::new();
    let mut run = Vec::new();
    let flush = |run: &mut Vec<&crate::db::Message>, output: &mut String| {
        if run.is_empty() {
            return;
        }
        if args.json {
            for message in run.iter() {
                output.push_str(&message_json(message));
                output.push('\n');
            }
        } else {
            output.push_str(&format!("\n{}\n", crate::cli::output::format_listen_messages(run, &spec.participant.canonical_name, &records)));
        }
        run.clear();
    };
    for message in &batch.messages {
        if is_match(message) {
            flush(&mut run, &mut output);
            if args.json {
                output.push_str(&format!("{}\n", serde_json::json!({"matched":true,"notification":message.text})));
            } else {
                output.push_str(&format!("\n{}\n", message.text));
            }
        } else {
            run.push(message);
        }
    }
    flush(&mut run, &mut output);
    if !filtered && !args.json {
        output.push_str(&batch.remaining_note(&spec.participant.canonical_name));
    }
    Ok((output, matched, human))
}

fn wait(session: &Session, args: &ListenArgs, spec: &ListenSpec, timeout: f64, notify: Option<&NotifyServer>, shutdown: &AtomicBool) -> ServiceResult<(i32, Option<ListenEnd>)> {
    let filtered = !spec.filters.is_empty() || spec.predicate.as_ref().is_some_and(|value| !value.is_empty());
    let start = Instant::now();
    let mut backlog_end = None;
    let mut saw_human = false;
    loop {
        if shutdown.load(Ordering::Relaxed) {
            if !args.json {
                eprintln!("\n[Signal received, shutting down]");
            }
            return Ok((130, None));
        }
        match call(session, Events::PulseListen { participant: spec.participant.clone(), id: spec.id.clone() })? {
            EventResult::ListenState { connected: false } => {
                if !args.json {
                    eprintln!("\n[Disconnected: AGENTX stopped for {}. Unless told otherwise, stop work and end your turn now]", spec.participant.canonical_name);
                }
                return Ok((0, None));
            }
            EventResult::ListenState { connected: true } => {}
            _ => return Err(ServiceError::Internal("daemon returned a different listener heartbeat".into())),
        }
        if let Some(batch) = Batch::prepare(&session.client, &spec.participant)? {
            crate::cli::context::claim_inline_delivery();
            let (output, matched, human) = batch_output(session, args, spec, &batch, filtered)?;
            if let Err(error) = batch.emit(&session.client, &output, false) {
                // Exact confirmation stays withheld after a failed flush. The
                // outer cleanup handles owned status without advancing cursors.
                let _ = retained_call(session, Events::EndListen { participant: spec.participant.clone(), id: spec.id.clone(), reason: Some(ListenEnd::DeliveryFailed) });
                return Err(error);
            }
            saw_human |= human;
            if matched || !filtered {
                // Standard text already includes the remaining hint.
                if !filtered && !args.json {
                    if batch.arrived_since(&session.client, &spec.participant)? {
                        write(&format!("[agentx] new message(s) arrived — run: agentx listen --iam {}\n", spec.participant.canonical_name))?;
                    }
                } else {
                    notes(session, args, spec, &batch)?;
                }
                return Ok((0, Some(if matched { ListenEnd::Match } else { ListenEnd::Message })));
            }
            let end = *backlog_end.get_or_insert(batch.backlog_end);
            let through = batch.messages.last().and_then(|message| message.event_id).map(crate::messaging::inbox::EventId);
            if batch.remaining > 0 && through < end {
                continue;
            }
            if saw_human {
                notes(session, args, spec, &batch)?;
                return Ok((0, Some(ListenEnd::Message)));
            }
        }
        // At least one unread check precedes every quick timeout, including
        // filter waits whose notification may be queued during setup.
        let elapsed = start.elapsed().as_secs_f64();
        if elapsed >= timeout {
            if !args.json {
                eprintln!("\n[Timeout: no {} after {timeout}s]", if filtered { "match" } else { "messages" });
            }
            return Ok((0, Some(ListenEnd::Timeout)));
        }
        let duration = Duration::from_secs_f64((timeout - elapsed).min(if notify.is_some() { 30.0 } else { 0.1 }));
        if let Some(server) = notify {
            server.wait(duration);
        } else {
            std::thread::sleep(duration);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::service::{Request as Participants, Response as ParticipantResult};
    use crate::service::{InvocationContext, client::Client};

    fn fixture() -> (tempfile::TempDir, crate::daemon::test_support::TestDaemon, Session) {
        let directory = tempfile::Builder::new().prefix("agentx-listen-client-").tempdir().unwrap();
        let database = directory.path().join("agentx.db");
        let db = crate::db::AgentxDb::open_at(&database).unwrap();
        db.save_instance_named("luna", serde_json::json!({"tool":"adhoc","created_at":100.0,"status":"inactive","wait_timeout":86400,"tcp_mode":0}).as_object().unwrap()).unwrap();
        drop(db);
        let daemon = crate::daemon::test_support::TestDaemon::start(directory.path(), &database);
        let invocation = InvocationContext { cwd: directory.path().into(), explicit_name: Some("luna".into()), ..Default::default() };
        let client = Client::connect_existing(directory.path(), invocation.clone()).unwrap();
        let Response::Participants(ParticipantResult::Actor(actor)) = client.call(Request::Participants(Participants::Resolve)).unwrap() else {
            panic!("actor");
        };
        let command = crate::shared::CommandContext { explicit_name: Some("luna".into()), identity: None, go: false, identity_warning: None };
        let session = Session { client, invocation, command, actor: Some(actor) };
        (directory, daemon, session)
    }

    #[test]
    fn native_listen_quiet_modes_use_ipc_and_preserve_persistent_wait() {
        use clap::Parser;
        let (_directory, _daemon, session) = fixture();
        for arguments in [vec!["listen", "1", "--json"], vec!["listen", "1", "--json", "--from", "nobody"]] {
            let args = ListenArgs::try_parse_from(arguments).unwrap();
            assert_eq!(cmd_listen(&session, &args), 0);
            let Response::Participants(ParticipantResult::Record(Some(row))) = session.client.call(Request::Participants(Participants::Record { name: "luna".into() })).unwrap() else {
                panic!("row");
            };
            assert_eq!(row.wait_timeout, Some(86400));
            assert_eq!(row.tcp_mode, 0);
            assert_eq!((row.status.as_str(), row.status_context.as_str()), ("inactive", "listen timeout"));
            assert!(row.last_stop > 0);
            let EventResult::Subscriptions(subs) = call(&session, Events::Subscriptions).unwrap() else {
                panic!("subs");
            };
            assert!(!subs.iter().any(|sub| sub["id"].as_str().is_some_and(|id| id.starts_with("listen-"))));
        }
    }

    #[test]
    fn native_listen_transport_failure_never_recreates_database() {
        use clap::Parser;
        let (directory, daemon, session) = fixture();
        drop(daemon);
        let database = directory.path().join("agentx.db");
        std::fs::remove_file(&database).unwrap();
        assert_eq!(cmd_listen(&session, &ListenArgs::try_parse_from(["listen", "1", "--json"]).unwrap()), 1);
        assert!(!database.exists());
    }
}
