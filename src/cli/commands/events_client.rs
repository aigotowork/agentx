//! Event observation keeps bounded native queries and caller-owned stream lifetime.
//! Read cursors are observation only; they never acknowledge a participant inbox.
use super::events::{self, EventsArgs, EventsSubArgs, EventsSubcmd, streamline_event};
use crate::agents::attachments::ParticipantRef;
use crate::cli::client::Session;
use crate::events::service::{EventPage, EventQuery, MAX_PAGE_EVENTS, ObserverSpec, Request as Events, Response as EventResult};
use crate::runtime::notify::NotifyServer;
use crate::service::{Request, Response, ServiceError, ServiceResult, client::Client};
use serde_json::{Value, json};
use std::io::Write;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

fn call(client: &Client, request: Events) -> ServiceResult<EventResult> {
    match client.call(Request::Events(request))? {
        Response::Events(response) => Ok(response),
        _ => Err(ServiceError::Internal("daemon returned a different event response".into())),
    }
}
fn page(client: &Client, query: EventQuery) -> ServiceResult<EventPage> {
    match call(client, Events::Query(query))? {
        EventResult::Page(page) => {
            for warning in &page.warnings {
                eprintln!("Warning: {warning}");
            }
            Ok(page)
        }
        _ => Err(ServiceError::Internal("daemon returned a different event page".into())),
    }
}
fn watermark(client: &Client) -> ServiceResult<i64> {
    match call(client, Events::Watermark)? {
        EventResult::Watermark { newest_id, .. } => Ok(newest_id),
        _ => Err(ServiceError::Internal("daemon returned a different event watermark".into())),
    }
}
fn error_code(error: &ServiceError) -> i32 {
    if matches!(error, ServiceError::InvalidInput(_)) { 2 } else { 1 }
}
fn emit(out: &mut impl Write, event: &Value, full: bool, filters: &crate::events::filters::FilterMap) -> bool {
    let value = if full { event.clone() } else { streamline_event(event, filters) };
    writeln!(out, "{value}").and_then(|_| out.flush()).is_ok()
}

fn reconciled_call(client: &Client, request: Events) -> ServiceResult<EventResult> {
    // Only these exact-lifetime operations promise idempotent reconciliation.
    let caller = crate::service::CallerContext {
        request_id: crate::service::RequestId(uuid::Uuid::new_v4().to_string()),
        invocation: client.invocation().clone(),
        deadline_unix_ms: None,
        expected_database_generation: Some(client.info().database_generation.clone()),
    };
    let request = Request::Events(request);
    let invoke = || client.call_with_context(caller.clone(), request.clone());
    let response = match invoke() {
        Err(ServiceError::OutcomeUnknown(_)) => invoke(),
        result => result,
    }?;
    match response {
        Response::Events(result) => Ok(result),
        _ => Err(ServiceError::Internal("daemon returned a different event response".into())),
    }
}

struct Observer {
    client: Client,
    spec: Option<ObserverSpec>,
    server: Option<NotifyServer>,
}

impl Observer {
    fn begin(client: &Client, participant: Option<&ParticipantRef>) -> ServiceResult<Self> {
        let mut observer = Self { client: client.clone(), spec: None, server: None };
        if let Some(participant) = participant {
            let server = NotifyServer::new().map_err(|error| ServiceError::Unavailable(error.to_string()))?;
            let spec = ObserverSpec { participant: participant.clone(), id: format!("observe-{}", uuid::Uuid::new_v4()), port: server.port() };
            match reconciled_call(client, Events::BeginObserver(spec.clone())) {
                Ok(EventResult::ObserverReady) => {}
                result => {
                    let _ = reconciled_call(client, Events::EndObserver(spec));
                    return Err(match result {
                        Err(error) => error,
                        _ => ServiceError::Internal("daemon returned a different observer setup".into()),
                    });
                }
            }
            observer.spec = Some(spec);
            observer.server = Some(server);
        }
        Ok(observer)
    }

    fn check(&self) -> ServiceResult<()> {
        if let Some(spec) = &self.spec {
            match call(&self.client, Events::CheckObserver(spec.clone()))? {
                EventResult::ObserverReady => {}
                _ => return Err(ServiceError::Internal("daemon returned a different observer state".into())),
            }
        }
        Ok(())
    }

    fn finish(&mut self) -> ServiceResult<()> {
        if let Some(spec) = self.spec.take() {
            match reconciled_call(&self.client, Events::EndObserver(spec))? {
                EventResult::ObserverEnded => {}
                _ => return Err(ServiceError::Internal("daemon returned a different observer cleanup".into())),
            }
        }
        Ok(())
    }

    fn pause(&self, remaining: Duration, shutdown: &AtomicBool, reader_gone: &impl Fn() -> bool) {
        let start = Instant::now();
        let tick = Duration::from_millis(250);
        let duration = remaining.min(if self.server.is_some() { Duration::from_secs(5) } else { tick });
        while start.elapsed() < duration && !shutdown.load(Ordering::Acquire) && !reader_gone() {
            let slice = duration.saturating_sub(start.elapsed()).min(tick);
            if let Some(server) = &self.server {
                if server.wait(slice) {
                    break;
                }
            } else {
                std::thread::sleep(slice);
            }
        }
    }
}

impl Drop for Observer {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

fn shutdown_flag() -> Arc<AtomicBool> {
    let shutdown = Arc::new(AtomicBool::new(false));
    crate::sys::signal::register_term(&shutdown);
    crate::sys::signal::register_int(&shutdown);
    shutdown
}

/// Freeze the current high-water mark before descending pages, then preserve
/// the command's timestamp/id ordering for the requested recent window.
fn recent(client: &Client, mut query: EventQuery, count: usize) -> ServiceResult<(Vec<Value>, crate::events::filters::FilterMap)> {
    let mut events = Vec::new();
    let mut history_id = None;
    query.oldest_first = false;
    if query.through.is_none() && !query.include_archives {
        query.through = Some(watermark(client)?);
    }
    let result = (|| {
        let resolved = loop {
            query.limit = count.saturating_sub(events.len()).min(MAX_PAGE_EVENTS);
            let result = page(client, query.clone())?;
            let resolved = result.resolved_filters;
            let oldest = result.events.iter().filter_map(|event| event["id"].as_i64()).min();
            if let Some(cursor) = &result.history {
                history_id = Some(cursor.snapshot.clone());
            }
            events.extend(result.events);
            if !result.has_more || events.len() >= count {
                break resolved;
            }
            query.warn_unknown_agents = false;
            if query.include_archives {
                query.history = Some(result.history.ok_or_else(|| ServiceError::Internal("history page did not supply a continuation boundary".into()))?);
            } else {
                let Some(oldest) = oldest else {
                    return Err(ServiceError::Internal("event page did not supply a continuation boundary".into()));
                };
                query.through = Some(oldest.saturating_sub(1));
            }
        };
        events.sort_by_cached_key(|event| (event["ts"].as_str().unwrap_or("").to_owned(), event["source"].as_str().unwrap_or("").to_owned(), event["id"].as_i64().unwrap_or(0)));
        Ok((events, resolved))
    })();
    let cleanup = history_id.map(|snapshot| reconciled_call(client, Events::EndHistory { snapshot })).transpose();
    let output = result?;
    if let Some(response) = cleanup?
        && !matches!(response, EventResult::HistoryEnded)
    {
        return Err(ServiceError::Internal("daemon returned a different history cleanup".into()));
    }
    Ok(output)
}

pub(crate) fn cmd_events(session: &Session, args: &EventsArgs) -> i32 {
    match run(session, args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error}");
            error_code(&error)
        }
    }
}
fn run(session: &Session, args: &EventsArgs) -> ServiceResult<i32> {
    if let Some(subcommand) = &args.subcmd {
        if args.remote_fetch {
            return Err(ServiceError::InvalidInput("--remote-fetch is only supported in query mode".into()));
        }
        return match subcommand {
            EventsSubcmd::Launch(args) => {
                let launcher = session.actor.as_ref().map(|actor| actor.record.name.as_str());
                let result = crate::runtime::launch_status::wait_for_launch_client(&session.client, launcher, args.batch_id.as_deref(), args.timeout).to_json();
                println!("{result}");
                Ok(match result["status"].as_str() {
                    Some("ready") => 0,
                    Some("timeout" | "blocked") => 2,
                    _ => 1,
                })
            }
            EventsSubcmd::Sub(args) => subscription(session, args),
            EventsSubcmd::Unsub(args) => {
                let id = if args.id.starts_with("sub-") { args.id.clone() } else { format!("sub-{}", args.id) };
                if let Some(device) = &args.device {
                    return Ok(events::cmd_events_unsub_remote(&session.client, device, &id));
                }
                match call(&session.client, Events::Unsubscribe { id: id.clone() })? {
                    EventResult::Unsubscribed(true) => {
                        println!("Removed {id}");
                        Ok(0)
                    }
                    EventResult::Unsubscribed(false) => {
                        eprintln!("Not found: {id}\nUse 'agentx events sub list' to list active subscriptions.");
                        Ok(1)
                    }
                    _ => Err(ServiceError::Internal("daemon returned a different unsubscribe response".into())),
                }
            }
        };
    }
    let conflict = if args.follow && args.wait.is_some() {
        Some("--follow cannot be combined with --wait")
    } else if args.follow && args.remote_fetch {
        Some("--follow cannot be combined with --remote-fetch")
    } else if args.follow && args.all {
        Some("--follow cannot be combined with --all")
    } else if args.follow && args.after_id.is_some() && args.last.is_some() {
        Some("--last and --after-id are mutually exclusive")
    } else if !args.follow && args.after_id.is_some() {
        Some("--after-id requires --follow")
    } else if args.remote_fetch && args.wait.is_some() {
        Some("--wait is not supported with --remote-fetch")
    } else {
        None
    };
    if let Some(message) = conflict {
        eprintln!("Error: {message}");
        return Ok(1);
    }
    let query = EventQuery {
        filters: args.filters.to_filter_map(),
        predicate: args.sql.as_ref().map(|sql| sql.replace("\\!", "!")),
        limit: args.last.unwrap_or(20),
        include_archives: args.all,
        warn_unknown_agents: !args.all && !args.follow && args.wait.is_none() && !args.remote_fetch,
        ..Default::default()
    };
    if args.remote_fetch {
        let device = args.device.as_deref().filter(|value| !value.is_empty()).ok_or_else(|| ServiceError::InvalidInput("--remote-fetch requires --device <SHORT_ID>".into()))?;
        let mut params = json!({"filters": query.filters, "last": query.limit});
        if let Some(sql) = &query.predicate {
            params["sql"] = json!(sql);
        }
        let result = crate::relay::control::dispatch_remote(&session.client, device, None, crate::relay::control::rpc_action::EVENTS, &params, crate::relay::control::RPC_DEFAULT_TIMEOUT).map_err(ServiceError::Unavailable)?;
        let events = result["events"].as_array().ok_or_else(|| ServiceError::Unavailable("Remote events fetch: malformed peer response (missing 'events' array)".into()))?;
        let mut out = std::io::stdout().lock();
        for event in events {
            if !emit(&mut out, event, args.full, &query.filters) {
                return Ok(0);
            }
        }
        for note in events::remote_truncation_notes(&result, events) {
            if writeln!(out, "{note}").and_then(|_| out.flush()).is_err() {
                return Ok(0);
            }
        }
        return Ok(0);
    }
    if args.follow {
        return follow(session, args, query);
    }
    if let Some(timeout) = args.wait {
        return wait(session, args, query, timeout);
    }
    let (events, filters) = recent(&session.client, query.clone(), query.limit)?;
    let mut out = std::io::stdout().lock();
    for event in &events {
        if !emit(&mut out, event, args.full, &filters) {
            return Ok(0);
        }
    }
    Ok(0)
}

fn follow(session: &Session, args: &EventsArgs, query: EventQuery) -> ServiceResult<i32> {
    let shutdown = shutdown_flag();
    follow_to(session, args, query, &mut std::io::stdout().lock(), &shutdown, &crate::sys::io::stdout_reader_gone)
}

fn reset_observation(client: &mut Client, observer: &mut Observer, out: &mut impl Write) -> ServiceResult<bool> {
    let _ = observer.finish();
    *client = Client::connect_existing(client.scope_root(), client.invocation().clone())?;
    // A retired generation's participant must never authorize its namesake.
    *observer = Observer::begin(client, None)?;
    Ok(writeln!(out, "{}", json!({"db_reset":true})).and_then(|_| out.flush()).is_ok())
}

fn follow_to(session: &Session, args: &EventsArgs, mut query: EventQuery, out: &mut impl Write, shutdown: &AtomicBool, reader_gone: &impl Fn() -> bool) -> ServiceResult<i32> {
    let mut client = session.client.clone();
    // Register before the watermark: an event in the setup/query gap is either
    // in the requested backlog or leaves a queued notification for the tail.
    let mut observer = Observer::begin(&client, session.actor.as_ref().map(|actor| &actor.participant))?;
    let result = (|| {
        let newest = loop {
            match watermark(&client) {
                Ok(newest) => break newest,
                Err(ServiceError::StaleFence) => {
                    if !reset_observation(&mut client, &mut observer, out)? {
                        return Ok(0);
                    }
                }
                Err(error) => return Err(error),
            }
        };
        let mut cursor = args.after_id.unwrap_or(newest);
        if cursor > newest {
            eprintln!("Error: --after-id {cursor} is past the newest event ({newest}); the database was likely reset. Rerun without --after-id.");
            return Ok(1);
        }
        if let Some(last) = args.last {
            query.through = Some(cursor);
            let (mut events, filters) = recent(&client, query.clone(), last)?;
            events.sort_by_key(|event| event["id"].as_i64().unwrap_or(0));
            for event in &events {
                if !emit(out, event, args.full, &filters) {
                    return Ok(0);
                }
            }
        }
        query.through = None;
        query.oldest_first = true;
        query.limit = 500;
        loop {
            if shutdown.load(Ordering::Acquire) {
                return Ok(130);
            }
            if reader_gone() {
                return Ok(0);
            }
            query.after = Some(cursor);
            let result = match page(&client, query.clone()) {
                Ok(result) => result,
                Err(ServiceError::StaleFence) => {
                    // Only a query generation rejection permits rebinding. An
                    // endpoint/participant rejection below ends this observer.
                    if !reset_observation(&mut client, &mut observer, out)? {
                        return Ok(0);
                    }
                    cursor = 0;
                    continue;
                }
                Err(error) => return Err(error),
            };
            if let Err(error) = observer.check() {
                if matches!(error, ServiceError::StaleFence) {
                    // A reset may commit between page and endpoint check. A
                    // second generation rejection distinguishes it from an
                    // exact participant/endpoint fence in the current scope.
                    match watermark(&client) {
                        Err(ServiceError::StaleFence) => {
                            if !reset_observation(&mut client, &mut observer, out)? {
                                return Ok(0);
                            }
                            cursor = 0;
                            continue;
                        }
                        Err(error) => return Err(error),
                        Ok(_) => {}
                    }
                }
                return Err(error);
            }
            for event in &result.events {
                if !emit(out, event, args.full, &result.resolved_filters) {
                    return Ok(0);
                }
            }
            cursor = result.next_after;
            if !result.has_more {
                observer.pause(Duration::from_secs(5), shutdown, reader_gone);
            }
        }
    })();
    let cleanup = observer.finish();
    let code = result?;
    cleanup?;
    Ok(code)
}

fn remaining(timeout: u64, elapsed: Duration) -> Duration {
    Duration::from_secs(timeout).saturating_sub(elapsed)
}

fn wait(session: &Session, args: &EventsArgs, query: EventQuery, timeout: u64) -> ServiceResult<i32> {
    let shutdown = shutdown_flag();
    wait_to(session, args, query, timeout, &mut std::io::stdout().lock(), &shutdown, &crate::sys::io::stdout_reader_gone)
}

fn wait_to(session: &Session, args: &EventsArgs, mut query: EventQuery, timeout: u64, out: &mut impl Write, shutdown: &AtomicBool, reader_gone: &impl Fn() -> bool) -> ServiceResult<i32> {
    let mut observer = Observer::begin(&session.client, session.actor.as_ref().map(|actor| &actor.participant))?;
    let start = Instant::now();
    let result = (|| {
        let mut cursor = watermark(&session.client)?;
        let cutoff = chrono::DateTime::from_timestamp(crate::shared::time::now_epoch_i64() - 10, 0).map(|time| time.to_rfc3339()).unwrap_or_default();
        let lookback = format!("timestamp > '{}'", cutoff.replace('\'', "''"));
        let predicate = query.predicate.clone();
        query.predicate = Some(match &predicate {
            Some(predicate) => format!("({predicate}) AND ({lookback})"),
            None => lookback,
        });
        query.include_archives = false;
        query.warn_unknown_agents = false;
        query.limit = 1;
        query.through = Some(cursor);
        let result = page(&session.client, query.clone())?;
        observer.check()?;
        if let Some(event) = result.events.first() {
            emit(out, event, args.full, &result.resolved_filters);
            return Ok(0);
        }
        query.predicate = predicate;
        query.through = None;
        query.oldest_first = true;
        let mut previewed = false;
        loop {
            if shutdown.load(Ordering::Acquire) {
                return Ok(130);
            }
            if reader_gone() {
                return Ok(0);
            }
            if remaining(timeout, start.elapsed()).is_zero() {
                let _ = writeln!(out, "{}", json!({"timed_out":true})).and_then(|_| out.flush());
                return Ok(1);
            }
            query.after = Some(cursor);
            let result = page(&session.client, query.clone())?;
            observer.check()?;
            cursor = result.next_after;
            if let Some(event) = result.events.first() {
                emit(out, event, args.full, &result.resolved_filters);
                return Ok(0);
            }
            if !previewed && let Some(actor) = &session.actor {
                match session.client.call(Request::Queries(crate::service::queries::Request::UnreadPreview { participant: actor.participant.clone() }))? {
                    Response::Queries(crate::service::queries::Response::UnreadPreview(Some(preview))) => {
                        if writeln!(out, "{preview}").and_then(|_| out.flush()).is_err() {
                            return Ok(0);
                        }
                        previewed = true;
                    }
                    Response::Queries(crate::service::queries::Response::UnreadPreview(None)) => {}
                    _ => return Err(ServiceError::Internal("daemon returned a different unread preview".into())),
                }
            }
            observer.pause(remaining(timeout, start.elapsed()), shutdown, reader_gone);
        }
    })();
    let cleanup = observer.finish();
    let code = result?;
    cleanup?;
    Ok(code)
}

fn subscription(session: &Session, args: &EventsSubArgs) -> ServiceResult<i32> {
    let is_list = args.rest.first().is_some_and(|value| value == "list");
    if let Some(device) = &args.device {
        return Ok(if is_list { events::cmd_events_sub_remote_list(&session.client, device) } else { events::cmd_events_sub_remote_create(&session.client, args, device) });
    }
    if is_list {
        return match call(&session.client, Events::Subscriptions)? {
            EventResult::Subscriptions(subs) => Ok(events::print_subscriptions(&subs)),
            _ => Err(ServiceError::Internal("daemon returned a different subscriptions projection".into())),
        };
    }
    let filters = args.filters.to_filter_map();
    if filters.is_empty() && args.rest.is_empty() {
        crate::cli::commands::help::print_command_help("events sub");
        return Ok(0);
    }
    let owner = if let Some(name) = &args.for_agent {
        let records = session.records()?;
        let record = records.iter().find(|record| record.name == *name).or_else(|| records.iter().find(|record| record.name.starts_with(name)));
        let Some(record) = record else {
            eprintln!("Not found: {name}\nUse 'agentx list' to see available agents");
            return Ok(1);
        };
        record.name.clone()
    } else if args.from_bigboss || args.as_name.is_some() {
        args.as_name.clone().unwrap_or_else(|| crate::shared::SENDER.into())
    } else if let Some(actor) = &session.actor {
        actor.record.name.clone()
    } else {
        eprintln!("Error: Cannot create subscription without identity.\nRun 'agentx start' first, or use --iam.");
        return Ok(1);
    };
    let sql = (!args.rest.is_empty()).then(|| args.rest.join(" "));
    let filter_based = !filters.is_empty();
    let created = match call(&session.client, Events::Subscribe { filters, predicate: sql, owner: owner.clone(), once: args.once, on_hit: args.on_hit.clone() })? {
        EventResult::Subscribed(created) => created,
        _ => return Err(ServiceError::Internal("daemon returned a different subscription result".into())),
    };
    let id = &created.id;
    if created.already_exists {
        println!("Subscription {id} already exists\n  stop: agentx events unsub {id}");
        return Ok(0);
    }
    if filter_based {
        println!("Subscription {id} created\n  stop: agentx events unsub {id}");
    } else {
        println!("{id}\n  for: {owner}\n  filter: {}\n  stop: agentx events unsub {id}", created.predicate.as_deref().unwrap_or(""));
    }
    if let Some(count) = created.historical_matches {
        if count > 0 {
            println!("  historical matches: {count} events");
            if !filter_based && let Some(event) = created.latest_match {
                let ts = event["ts"].as_str().unwrap_or("");
                println!("  latest match: [{}] {} @ {}", event["type"].as_str().unwrap_or(""), event["instance"].as_str().unwrap_or(""), ts.get(..19).unwrap_or(ts));
            }
            println!("  You will be notified on the next matching event(s)");
        } else if !filter_based {
            println!("  historical matches: 0 (filter will apply to future events only)");
        }
    }
    crate::cli::tips::maybe_show_tip_client(&session.client, &owner, "sub:created", false)?;
    Ok(0)
}

#[cfg(test)]
#[path = "events_client_tests.rs"]
mod tests;
