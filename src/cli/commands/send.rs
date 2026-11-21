//! `agentx send` command — send messages to agentx instances.
mod delivery;
mod directory;

use std::io::{IsTerminal, Read as IoRead};

#[cfg(test)]
use crate::agents::identity;
use crate::cli::client::Session;
#[cfg(test)]
use crate::cli::context::InlineBatch;
#[cfg(test)]
use crate::db::AgentxDb;
#[cfg(test)]
use crate::messaging::routing::MessageScope;
use crate::messaging::routing::{MessageEnvelope, validate_intent, validate_message};
use crate::messaging::service::{Request as Messages, Response as MessageResult, Submission};
#[cfg(test)]
use crate::messaging::submission::{get_thread_from_event, resolve_delivery, resolve_reply_to_local};
use crate::service::{Request, Response, ServiceError};
#[cfg(test)]
use crate::shared::{CommandContext, SenderIdentity, SenderKind, is_inside_ai_tool};
use crate::shared::{SENDER, status_icon};

const SEND_AFTER_HELP: &str = "\
Target matching:
    @luna                          exact local name
    @luna:BOXE                     exact or uniquely prefixed remote agent
  Partial local names are rejected to avoid accidental fan-out.

Inline bundle (attach structured context):
    --title <text>                 Create and attach bundle inline
    --description <text>           Bundle description (required with --title)
    --events <ids>                 Event IDs/ranges: 1,2,5-10
    --files <paths>                Comma-separated file paths
    --transcript <ranges>          Format: 3-14:normal,6:full,22-30:detailed
    --extends <id>                 Parent bundle (optional)
  See 'agentx bundle --help' for bundle details

Examples:
    agentx send @luna -- Hello there!
    agentx send @luna @nova --intent request -- Can you help?
    agentx send -- Broadcast message to everyone
    echo 'Complex message' | agentx send @luna
    agentx send @luna <<'EOF'
    Multi-line message with special chars
    EOF";

/// Parse positional arg: accept both @targets and bare text.
/// Bare text (no @ prefix) is accepted by clap and separated later in cmd_send.
fn parse_positional(s: &str) -> Result<String, String> {
    if s == "@" { Err("Empty target '@' is not allowed".to_string()) } else { Ok(s.to_string()) }
}

/// Parsed arguments for `agentx send`.
#[derive(clap::Parser, Debug)]
#[command(
    name = "send",
    about = "Send a message to agents",
    after_help = SEND_AFTER_HELP,
)]
pub struct SendArgs {
    /// Read delivery facts for an accepted event without sending again.
    #[arg(long, conflicts_with_all = ["positionals", "message", "stdin", "file", "base64", "dir", "workspace", "dry_run"])]
    pub delivery_status: Option<i64>,
    /// Address the default recipient of this directory's workspace.
    #[arg(long)]
    pub dir: Option<String>,
    /// Select an exact workspace directory, or global.
    #[arg(long)]
    pub workspace: Option<String>,
    /// Inspect directory routing without starting an agent or sending a message.
    #[arg(long, conflicts_with_all = ["quiet", "reply_to", "thread"])]
    pub dry_run: bool,
    /// Positional args: @targets and/or bare message text (backward compat)
    #[arg(value_parser = parse_positional)]
    pub positionals: Vec<String>,

    /// Message text (after --)
    #[arg(last = true)]
    pub message: Vec<String>,

    // ── Message source ──
    /// Read message from stdin
    #[arg(long)]
    pub stdin: bool,

    /// Read message from file
    #[arg(long)]
    pub file: Option<String>,

    /// Read message from base64-encoded string
    #[arg(long)]
    pub base64: Option<String>,

    // ── Envelope ──
    /// Message intent (request|inform|ack)
    #[arg(long)]
    pub intent: Option<String>,

    /// Reply to event ID (42 or 42:BOXE)
    #[arg(long)]
    pub reply_to: Option<String>,

    /// Threaded routing: seed recipients once, then reuse thread members
    #[arg(long)]
    pub thread: Option<String>,

    // ── Sender ──
    /// External sender identity
    #[arg(long)]
    pub from: Option<String>,

    /// Shorthand for --from bigboss
    #[arg(short = 'b')]
    pub bigboss: bool,

    /// No output; leaves mail unread
    #[arg(long)]
    pub quiet: bool,

    /// JSON receipt; leaves mail unread
    #[arg(long)]
    pub json: bool,

    // ── Inline bundle ──
    /// Bundle title (creates inline bundle)
    #[arg(long)]
    pub title: Option<String>,

    /// Bundle description
    #[arg(long)]
    pub description: Option<String>,

    /// Bundle event IDs/ranges
    #[arg(long)]
    pub events: Option<String>,

    /// Bundle file paths (comma-separated)
    #[arg(long, rename_all = "verbatim")]
    pub files: Option<String>,

    /// Bundle transcript ranges
    #[arg(long)]
    pub transcript: Option<String>,

    /// Parent bundle ID
    #[arg(long)]
    pub extends: Option<String>,

    /// Set by router: whether `--` was present in raw argv.
    /// Clap can't distinguish "no --" from "-- with no args", so the router sets this.
    #[arg(skip)]
    pub had_separator: bool,
}

impl SendArgs {
    /// Resolve the effective --from name (--from overrides -b).
    pub(crate) fn sender_name(&self) -> Option<String> {
        if let Some(ref name) = self.from {
            Some(name.clone())
        } else if self.bigboss {
            Some("bigboss".to_string())
        } else {
            None
        }
    }

    /// Whether a `--` separator was present in the raw argv.
    fn has_separator(&self) -> bool {
        self.had_separator
    }

    /// Build inline bundle data from flags, or None if no bundle flags present.
    fn build_bundle_data(&self) -> Result<Option<serde_json::Value>, String> {
        let has_any = self.title.is_some() || self.description.is_some() || self.events.is_some() || self.files.is_some() || self.transcript.is_some() || self.extends.is_some();

        if !has_any {
            return Ok(None);
        }

        let title = self.title.as_ref().ok_or_else(|| {
            let present: Vec<&str> =
                [self.description.as_ref().map(|_| "--description"), self.events.as_ref().map(|_| "--events"), self.files.as_ref().map(|_| "--files"), self.transcript.as_ref().map(|_| "--transcript"), self.extends.as_ref().map(|_| "--extends")]
                    .into_iter()
                    .flatten()
                    .collect();
            format!("Bundle flags require --title: found {} without --title", present.join(", "))
        })?;

        let description = self.description.as_ref().ok_or("--description is required when --title is present")?;

        use crate::messaging::bundles::parse_csv_list;
        let events = parse_csv_list(self.events.as_deref());
        let files = parse_csv_list(self.files.as_deref());
        let transcript = parse_csv_list(self.transcript.as_deref());

        let mut bundle = serde_json::json!({
            "title": title,
            "description": description,
            "refs": {
                "events": events,
                "files": files,
                "transcript": transcript,
            }
        });

        if let Some(ref ext) = self.extends {
            bundle.as_object_mut().unwrap().insert("extends".into(), serde_json::json!(ext));
        }

        Ok(Some(bundle))
    }
}

/// Get formatted recipient feedback showing who received the message.
///
/// `thread` is set when delivery was limited to that thread's members.
#[cfg(test)]
fn get_recipient_feedback(db: &AgentxDb, delivered_to: &[String], thread: Option<&str>) -> String {
    if delivered_to.is_empty() {
        return match thread {
            Some(thread) => {
                format!("Sent to: {SENDER} only (no other members of thread '{thread}' are active)")
            }
            None => format!("Sent to: {SENDER} only (no other agents are active)"),
        };
    }
    if delivered_to.len() > 10 {
        return format!("Sent to {} agents", delivered_to.len());
    }

    let mut parts = Vec::new();
    for name in delivered_to {
        if let Ok(Some(data)) = db.get_instance_full(name) {
            let icon = status_icon(&data.status);
            let display = name;
            parts.push(format!("{icon} {display}"));
        } else {
            parts.push(format!("◌ {name}"));
        }
    }
    format!("Sent to: {}", parts.join(", "))
}

#[cfg(test)]
fn print_broadcast_preview(db: &AgentxDb, delivered_to: &[String]) {
    let count = delivered_to.len();
    let names: Vec<String> = delivered_to
        .iter()
        .map(|name| {
            if let Ok(Some(data)) = db.get_instance_full(name) {
                let icon = status_icon(&data.status);
                let display = name;
                format!("{icon} {display}")
            } else {
                format!("◌ {name}")
            }
        })
        .collect();
    let recipient_list = if count <= 12 { names.join(", ") } else { format!("{} ... (+{} more)", names[..10].join(", "), count - 10) };

    println!("\n== BROADCAST SEND PREVIEW ==");
    println!("This message has no @targets, so it would broadcast to {count} agents.");
    println!("Recipients:\n  {recipient_list}\n");
    println!("Did you mean to send this to everyone?");
    println!("Broadcasts can wake many terminals and spend many agents' context/tools.");
    println!("\nAdd --go after send and run again to proceed:");
    println!("  agentx send --go ...\n");
}

/// Persist one message atomically, then publish advisory wake notifications.
/// Daemon clients use the stable request-id acceptance contract instead.
#[cfg(test)]
pub fn send_message(db: &AgentxDb, identity: &SenderIdentity, message: &str, envelope: Option<&MessageEnvelope>, explicit_targets: Option<&[String]>) -> Result<(i64, Vec<String>), String> {
    let persisted = db.with_write_scope(|| crate::messaging::submission::persist_message(db, identity, message, envelope, explicit_targets).map_err(anyhow::Error::msg)).map_err(|error| error.to_string())?;
    db.after_event_logged(persisted.event_id, "message", &persisted.routing_instance, &persisted.data);
    crate::runtime::notify::wake_all(db);
    crate::relay::trigger_push();
    Ok((persisted.event_id, persisted.recipients))
}

/// Resolve message from one of 5 source modes; text after `--` is literal.
pub(crate) fn resolve_message(args: &SendArgs) -> Result<String, String> {
    let has_separator = args.has_separator();

    // Mutual exclusivity
    let source_count = [args.stdin, args.file.is_some(), args.base64.is_some(), has_separator].iter().filter(|&&x| x).count();
    if source_count > 1 {
        return Err("Only one of --, --stdin, --file, --base64 can be used".to_string());
    }

    // 1. -- separator
    if has_separator {
        let text = args.message.join(" ");
        if text.is_empty() {
            return Err("No message after --".to_string());
        }
        return Ok(text);
    }

    // 2. --stdin
    if args.stdin {
        return read_stdin();
    }

    // 3. --file
    if let Some(ref path) = args.file {
        let resolved = if std::path::Path::new(path).is_absolute() { std::path::PathBuf::from(path) } else { std::env::current_dir().unwrap_or_default().join(path) };
        return match std::fs::read_to_string(&resolved) {
            Ok(content) if !content.is_empty() => Ok(content),
            Ok(_) => Err(format!("File is empty: {path}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(format!("File not found: {path}")),
            Err(e) => Err(format!("Cannot read file: {e}")),
        };
    }

    // 4. --base64
    if let Some(ref b64) = args.base64 {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64).map_err(|_| "Invalid base64 encoding".to_string())?;
        let s = String::from_utf8(bytes).map_err(|_| "Base64 decoded to invalid UTF-8".to_string())?;
        if s.is_empty() {
            return Err("Base64 decoded to empty string".to_string());
        }
        return Ok(s);
    }

    // 5. Auto-pipe (stdin is a pipe, no explicit source)
    if !std::io::stdin().is_terminal() {
        return read_stdin();
    }

    // No message source found
    let targets_str = if args.positionals.is_empty() { "@target".to_string() } else { args.positionals.iter().take(3).map(|t| if t.starts_with('@') { t.clone() } else { format!("@{t}") }).collect::<Vec<_>>().join(" ") };
    Err(format!("No message provided.\nUse: agentx send {targets_str} -- your message\n Or: echo 'msg' | agentx send {targets_str}"))
}

/// Read message from stdin pipe.
fn read_stdin() -> Result<String, String> {
    let mut buf = String::new();
    if std::io::stdin().read_to_string(&mut buf).is_ok() && !buf.is_empty() { Ok(buf) } else { Err("No input received on stdin".to_string()) }
}

/// Process positional args without `--` separator.
/// Matches Python messaging.py behavior:
///   - Empty → ([], None)
///   - Single arg with `@` prefix and space → backward compat: entire text is message
///   - Mix of @targets and bare text → separate targets from message
///   - Pure @targets → targets only, no message
pub(crate) fn process_positionals(positionals: &[String]) -> (Vec<String>, Option<String>) {
    if positionals.is_empty() {
        return (vec![], None);
    }

    // Backward compat: single arg starting with @ and containing space
    // e.g. "@luna hi" → whole thing is message (compute_scope extracts @mentions)
    if positionals.len() == 1 && positionals[0].starts_with('@') && positionals[0].contains(' ') {
        return (vec![], Some(positionals[0].clone()));
    }

    // Separate @targets from bare text
    let mut targets = Vec::new();
    let mut remaining = Vec::new();

    for arg in positionals {
        if let Some(name) = arg.strip_prefix('@') {
            targets.push(name.to_string());
        } else {
            remaining.push(arg.clone());
        }
    }

    if remaining.len() > 1 {
        // Multiple non-@ args without -- separator → error
        // Return empty message to trigger "no message" error with helpful hint
        return (targets, None);
    }

    if remaining.len() == 1 {
        return (targets, Some(remaining[0].clone()));
    }

    (targets, None)
}

/// After a failed send, say which requested targets are stopped (and when) so
/// the sender can resume them instead of guessing at typos.
#[cfg(test)]
fn print_stopped_target_hints(db: &AgentxDb, explicit_targets: &[String], message: &str) {
    let targets = if explicit_targets.is_empty() { crate::shared::constants::extract_mentions(message) } else { explicit_targets.to_vec() };
    for target in targets {
        if let Some(stopped) = identity::last_stopped(db, &target) {
            eprintln!("  @{target} {} — resume: agentx r {}", stopped.summary(), stopped.name);
        }
    }
}

/// Reject `agentx send hi` / `agentx send luna`: a lone bare word with no `@` and
/// no `--` is as likely a target missing its `@` as a broadcast, and guessing
/// broadcast interrupts every agent. Quoted phrases stay broadcasts.
#[cfg(test)]
fn ambiguous_bare_word_error(db: &AgentxDb, targets: &[String], word: &str) -> Option<String> {
    if !targets.is_empty() || word.split_whitespace().nth(1).is_some() {
        return None;
    }
    let mut msg = format!("Error: '{word}' is ambiguous: a target needs '@', a broadcast needs '--'");
    if db.get_instance_full(word).ok().flatten().is_some() || identity::last_stopped(db, word).is_some() {
        msg.push_str(&format!("\nDid you mean @{word}? agentx send @{word} -- <message>"));
    } else {
        msg.push_str(&format!("\n  Direct:    agentx send @name -- {word}\n  Broadcast: agentx send -- {word}"));
    }
    Some(msg)
}

/// Main entry point for `agentx send` command.
///
/// Returns exit code (0 = success, 1 = error).
pub(crate) fn cmd_send_client(session: &Session, args: &SendArgs) -> i32 {
    match send_client(session, args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error}");
            1
        }
    }
}

fn send_client(session: &Session, args: &SendArgs) -> Result<i32, String> {
    if let Some(event) = args.delivery_status {
        let report = delivery::read(session, event)?;
        if args.json {
            println!("{}", serde_json::to_string(&report).map_err(|error| error.to_string())?);
        } else {
            println!("{}", delivery::render(&report));
        }
        return Ok(0);
    }
    if args.dry_run {
        if args.dir.is_none() && args.workspace.is_none() {
            return Err("send --dry-run requires --dir or --workspace".into());
        }
        if !args.positionals.is_empty() {
            return Err("send --dry-run inspects a directory endpoint; omit explicit recipients".into());
        }
        let (address, _) = directory::resolve(session, args.dir.as_deref(), args.workspace.as_deref()).map_err(|error| format!("{error:#}"))?;
        println!("{}", serde_json::to_string(&address).map_err(|error| error.to_string())?);
        return Ok(0);
    }
    let from = args.sender_name();
    if let Some(name) = &from {
        if name.is_empty() || name.len() > 50 {
            return Err(format!("Name too long ({} chars, max 50)", name.len()));
        }
        if name.contains(['@', '|', '&', ';', '<', '>', '`', '$', '\'', '"', '\\', '\n', '\r']) {
            return Err("Name contains invalid characters".into());
        }
    }
    let mut envelope = MessageEnvelope::default();
    if let Some(intent) = &args.intent {
        let intent = intent.to_lowercase();
        validate_intent(&intent)?;
        envelope.intent = intent.parse().ok();
    }
    envelope.reply_to = args.reply_to.clone();
    envelope.thread = args.thread.clone();
    if envelope.intent.as_ref().is_some_and(|intent| intent.as_str() == "ack") && envelope.reply_to.is_none() {
        return Err("Intent 'ack' requires --reply-to <id>\n<id> is the number in received messages like [request #id]".into());
    }
    let bundle = args.build_bundle_data()?;
    if let Some(bundle) = &bundle {
        crate::messaging::bundles::warn_missing_files_for_cwd(bundle, &session.invocation.cwd);
    }
    let (mut targets, compat_message) = if !args.has_separator() && !args.stdin && args.file.is_none() && args.base64.is_none() {
        let (targets, message) = process_positionals(&args.positionals);
        if let Some(word) = message.as_deref()
            && targets.is_empty()
            && word.split_whitespace().nth(1).is_none()
        {
            let stopped = stopped_targets(session, vec![word.to_string()])?;
            let known = !stopped.is_empty() || matches!(
                session.client.call(Request::Participants(crate::agents::service::Request::Lookup { name: word.into() })).map_err(|error| error.to_string())?,
                Response::Participants(crate::agents::service::Response::Participant(Some(_)))
            );
            let hint = if known { format!("\nDid you mean @{word}? agentx send @{word} -- <message>") } else { format!("\n  Direct:    agentx send @name -- {word}\n  Broadcast: agentx send -- {word}") };
            return Err(format!("'{word}' is ambiguous: a target needs '@', a broadcast needs '--'{hint}"));
        }
        (targets, message)
    } else {
        let mut targets = Vec::new();
        for argument in &args.positionals {
            match argument.strip_prefix('@') {
                Some("") => return Err("Empty target '@' is not allowed".into()),
                Some(target) => targets.push(target.to_owned()),
                None => {
                    let hint = if argument.chars().all(|character| character.is_alphabetic()) && argument.len() <= 20 { format!("\nDid you mean @{argument}? Targets require @") } else { String::new() };
                    return Err(format!("Unexpected argument '{argument}'{hint}"));
                }
            }
        }
        (targets, None)
    };
    let message = if let Some(message) = compat_message { message } else { resolve_message(args)? };
    validate_message(&message)?;
    let directory_selected = args.dir.is_some() || args.workspace.is_some();
    let mut directory_address = None;
    if directory_selected && targets.is_empty() {
        let (mut address, _) = directory::resolve(session, args.dir.as_deref(), args.workspace.as_deref()).map_err(|error| format!("{error:#}"))?;
        let recipient = directory::recipient(session, &mut address).map_err(|error| format!("{error:#}"))?;
        targets.push(recipient);
        envelope.requested_directory = Some(address.requested_directory.clone());
        directory_address = Some(address);
    } else if directory_selected {
        let selected = crate::cli::workspace::selected(session, args.dir.as_deref(), args.workspace.as_deref()).map_err(|error| format!("{error:#}"))?;
        let selected = selected.as_ref().map(|path| path.to_string_lossy().into_owned());
        let records = session.records().map_err(|error| error.to_string())?;
        for target in &targets {
            let record = records.iter().find(|record| record.name == *target).ok_or_else(|| format!("directory-scoped recipients must be exact local agent names: @{target}"))?;
            if !crate::cli::workspace::visible(record, selected.as_deref()) {
                return Err(format!("@{target} does not belong to the selected workspace"));
            }
        }
    }
    let runtime = crate::runtime::context::AgentxContext::from_env(&session.invocation.environment, session.invocation.cwd.clone());
    let submission = Submission { recipients: targets.clone(), text: message.clone(), envelope, external_sender: from.clone(), broadcast_confirmed: session.command.go || !runtime.is_inside_ai_tool(), inline_bundle: bundle };
    let intent = submission.envelope.intent;
    let show_thread = !args.json && !args.quiet && (submission.envelope.thread.is_some() || submission.envelope.reply_to.is_some());
    // Explicit targets either resolve to mentions or are rejected in Submit.
    let thread = if targets.is_empty() && (!submission.broadcast_confirmed || show_thread) {
        let preview = match session.client.call(Request::Messages(Messages::Preview(submission.clone()))) {
            Ok(Response::Messages(MessageResult::Preview(preview))) => preview,
            Err(error) => return rejected_send(session, error, targets, &message),
            _ => return Err("daemon returned a different message preview".into()),
        };
        if !submission.broadcast_confirmed && preview.broadcast && !preview.thread_resolved && preview.recipients.len() > 3 {
            let records = session.records().map_err(|error| error.to_string())?;
            print_native_broadcast_preview(&records, &preview.recipients);
            return Ok(1);
        }
        preview.normalized_envelope.thread.filter(|_| preview.thread_resolved)
    } else {
        None
    };
    let request_id = crate::service::RequestId(uuid::Uuid::new_v4().to_string());
    let caller = crate::service::CallerContext { request_id: request_id.clone(), invocation: session.invocation.clone(), deadline_unix_ms: None, expected_database_generation: Some(session.client.info().database_generation.clone()) };
    let acceptance = match session.client.call_with_context(caller, Request::Messages(Messages::Submit(submission))) {
        Ok(Response::Messages(MessageResult::Accepted(acceptance))) => acceptance,
        Err(ServiceError::OutcomeUnknown(detail)) => {
            // Lookup the committed domain acceptance; never resend after a lost reply.
            match session.client.call(Request::Messages(Messages::Acceptance { request_id: request_id.clone() })) {
                Ok(Response::Messages(MessageResult::Acceptance(Some(acceptance)))) => acceptance,
                _ => return Err(format!("message outcome unknown (request {}); reconcile before retry: {detail}", request_id.0)),
            }
        }
        Err(error @ ServiceError::InvalidInput(_)) => return rejected_send(session, error, targets, &message),
        Err(error) => return Err(error.to_string()),
        _ => return Err("daemon returned a different message acceptance".into()),
    };
    crate::cli::context::claim_inline_delivery();
    crate::relay::worker::try_spawn_worker();
    // Acceptance is already durable. A failed observation cannot turn it into
    // a failed submission or authorize resending the same message.
    let report = if args.quiet { None } else { Some(delivery::read(session, acceptance.event.0)) };
    if args.json {
        let delivery = match report {
            Some(Ok(report)) => serde_json::to_value(report).map_err(|error| error.to_string())?,
            Some(Err(error)) => serde_json::json!({"state":"unavailable", "detail":error}),
            None => serde_json::Value::Null,
        };
        println!("{}", serde_json::json!({"event_id":acceptance.event.0,"delivered_to":acceptance.recipients,"directory_route":directory_address,"delivery":delivery}));
        return Ok(0);
    }
    if args.quiet {
        return Ok(0);
    }
    if let Some(address) = &directory_address {
        println!("Directory: {} → {} ({}){}", address.requested_directory, address.recipient.as_deref().unwrap_or("?"), address.resolved_workspace.as_deref().unwrap_or("global"), if address.created { " [created]" } else { " [reused]" });
    }
    let mut feedback = native_recipient_feedback(report.as_ref().and_then(|result| result.as_ref().ok()), &acceptance.recipients, thread.as_deref());
    if let Some(report) = report {
        feedback.push('\n');
        feedback.push_str(&match report {
            Ok(report) => delivery::render(&report),
            Err(error) => format!("[delivery #{}] accepted; delivery observation unavailable: {error}. Do not resend; inspect with agentx send --delivery-status {}", acceptance.event.0, acceptance.event.0),
        });
    }
    let receiver = session.actor.as_ref().filter(|actor| actor.record.tool == "adhoc");
    let mut received = None;
    if let Some(receiver) = receiver {
        if let Some(batch) = crate::cli::output::Batch::prepare(&session.client, &receiver.participant).map_err(|error| format!("Message sent, but {error}"))? {
            let records = session.records().map_err(|error| format!("Message sent, but {error}"))?;
            let (child_messages, main_messages): (Vec<_>, Vec<_>) = batch.messages.iter().partition(|message| records.iter().any(|record| record.name == message.from && record.parent_name.as_deref() == Some(&receiver.record.name)));
            let mut output = feedback.clone();
            if !main_messages.is_empty() {
                output.push('\n');
                output.push_str(&crate::cli::output::format_messages(&main_messages, &receiver.record.name, &records));
            }
            if !child_messages.is_empty() {
                output.push_str("\n[Subagent messages]\n");
                output.push_str(&crate::cli::output::format_messages(&child_messages, &receiver.record.name, &records));
            }
            output.push('\n');
            output.push_str(&batch.remaining_note(&receiver.record.name));
            batch.emit(&session.client, &output, from.is_some()).map_err(|error| format!("Message sent, but {error}"))?;
            received = Some(batch);
        } else {
            println!("{feedback}");
        }
    } else {
        println!("{feedback}");
    }
    if let Some(receiver) = receiver {
        let arrived = match &received {
            Some(batch) => batch.arrived_since(&session.client, &receiver.participant),
            None => crate::cli::output::pending_after(&session.client, &receiver.participant, crate::messaging::inbox::EventId(0)),
        }
        .map_err(|error| format!("Message sent, but {error}"))?;
        if arrived {
            println!("[agentx] new message(s) arrived — run: agentx listen --iam {}", receiver.record.name);
        }
    }
    if from.is_none()
        && let (Some(intent), Some(actor)) = (intent, &session.actor)
    {
        crate::cli::tips::maybe_show_tip_client(&session.client, &actor.record.name, &format!("send:intent:{}", intent.as_str()), false).map_err(|error| format!("Message sent, but {error}"))?;
    }
    Ok(0)
}

fn rejected_send(session: &Session, error: ServiceError, targets: Vec<String>, message: &str) -> Result<i32, String> {
    eprintln!("Error: {error}");
    let targets = if targets.is_empty() { crate::shared::constants::extract_mentions(message) } else { targets };
    if !targets.is_empty() {
        for stopped in stopped_targets(session, targets)? {
            eprintln!("  @{} {} — resume: agentx r {}", stopped.name, stopped.summary(), stopped.name);
        }
    }
    Ok(1)
}

fn stopped_targets(session: &Session, targets: Vec<String>) -> Result<Vec<crate::agents::identity::StoppedAgent>, String> {
    match session.client.call(Request::Queries(crate::service::queries::Request::StoppedTargets { targets })).map_err(|error| error.to_string())? {
        Response::Queries(crate::service::queries::Response::StoppedTargets(stopped)) => Ok(stopped),
        _ => Err("daemon returned a different stopped-participant projection".into()),
    }
}
fn native_recipient_feedback(report: Option<&crate::messaging::delivery_report::DeliveryReport>, targets: &[String], thread: Option<&str>) -> String {
    if targets.is_empty() {
        return thread.map(|thread| format!("Sent to: {SENDER} only (no other members of thread '{thread}' are active)")).unwrap_or_else(|| format!("Sent to: {SENDER} only (no other agents are active)"));
    }
    if targets.len() > 10 {
        return format!("Sent to {} agents", targets.len());
    }
    let labels = targets.iter().map(|name| {
        let status = report.and_then(|report| report.recipients.iter().find(|recipient| &recipient.name == name)).and_then(|recipient| recipient.status.as_deref());
        format!("{} {name}", status.map(status_icon).unwrap_or("◌"))
    }).collect::<Vec<_>>();
    format!("Sent to: {}", labels.join(", "))
}
fn recipient_labels(records: &[crate::db::InstanceRow], targets: &[String]) -> Vec<String> {
    targets.iter().map(|name| records.iter().find(|record| &record.name == name).map(|record| format!("{} {}", status_icon(&record.status), record.name)).unwrap_or_else(|| format!("◌ {name}"))).collect()
}
fn print_native_broadcast_preview(records: &[crate::db::InstanceRow], targets: &[String]) {
    let count = targets.len();
    let labels = recipient_labels(records, targets);
    let recipients = if count <= 12 { labels.join(", ") } else { format!("{} ... (+{} more)", labels[..10].join(", "), count - 10) };
    println!(
        "\n== BROADCAST SEND PREVIEW ==\nThis message has no @targets, so it would broadcast to {count} agents.\nRecipients:\n  {recipients}\n\nDid you mean to send this to everyone?\nBroadcasts can wake many terminals and spend many agents' context/tools.\n\nAdd --go after send and run again to proceed:\n  agentx send --go ...\n"
    );
}

#[cfg(test)]
pub fn cmd_send(db: &AgentxDb, args: &SendArgs, ctx: Option<&CommandContext>) -> i32 {
    // ── Resolve --from name ──
    let from_name = args.sender_name();

    if let Some(ref name) = from_name {
        if name.is_empty() || name.len() > 50 {
            eprintln!("Error: Name too long ({} chars, max 50)", name.len());
            return 1;
        }
        if name.contains(['@', '|', '&', ';', '<', '>', '`', '$', '\'', '"', '\\', '\n', '\r']) {
            eprintln!("Error: Name contains invalid characters");
            return 1;
        }
    }

    // Guard: subagents cannot use --from/-b
    if from_name.is_some() {
        let actor_from_ctx = ctx.and_then(|c| c.identity.clone());
        let actor = actor_from_ctx.or_else(|| identity::resolve_identity(db, None, None, None, None, None).ok());
        match actor {
            Some(ref actor) if matches!(actor.kind, SenderKind::Instance) => {
                if let Some(ref data) = actor.instance_data
                    && data.get("parent_name").and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty())
                {
                    eprintln!("Error: Subagents cannot use --from/-b (external sender spoofing)");
                    return 1;
                }
            }
            _ => {}
        }
    }

    let explicit_name = ctx.and_then(|c| c.explicit_name.as_deref());

    // ── Validate envelope flags ──
    let mut envelope = MessageEnvelope::default();

    if let Some(ref val) = args.intent {
        let val = val.to_lowercase();
        if let Err(e) = validate_intent(&val) {
            eprintln!("Error: {e}");
            return 1;
        }
        envelope.intent = val.parse().ok();
    }

    envelope.reply_to = args.reply_to.clone();

    if let Some(ref val) = args.thread {
        if val.len() > 64 {
            eprintln!("Error: Thread name too long ({} chars, max 64)", val.len());
            return 1;
        }
        if !val.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            eprintln!("Error: Thread name must be alphanumeric with hyphens/underscores");
            return 1;
        }
        envelope.thread = Some(val.clone());
    }

    // Ack requires reply_to
    if envelope.intent.as_ref().map(|i| i.as_str()) == Some("ack") && envelope.reply_to.is_none() {
        eprintln!("Error: Intent 'ack' requires --reply-to <id>");
        eprintln!("<id> is the number in received messages like [request #id]");
        return 1;
    }

    if let Some(ref reply_to) = envelope.reply_to {
        if let Some(local_id) = resolve_reply_to_local(db, reply_to) {
            if envelope.thread.is_none()
                && let Some(parent_thread) = get_thread_from_event(db, local_id)
            {
                envelope.thread = Some(parent_thread);
            }
        } else {
            eprintln!("Error: Invalid --reply-to: event not found or not a message");
            return 1;
        }
    }

    // ── Inline bundle ──
    let bundle_data = match args.build_bundle_data() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };

    // ── Process positional args: separate @targets from bare message text ──
    // Matches Python behavior in messaging.py:
    //   - Single "@name message" (with space) → entire text is message, @mention parsed by compute_scope
    //   - Non-@ args → message text (broadcast)
    //   - Pure @targets → explicit targets
    let (effective_targets, compat_message) = if !args.has_separator() && !args.stdin && args.file.is_none() && args.base64.is_none() {
        let (targets, message) = process_positionals(&args.positionals);
        if let Some(word) = message.as_deref()
            && let Some(err) = ambiguous_bare_word_error(db, &targets, word)
        {
            eprintln!("{err}");
            return 1;
        }
        (targets, message)
    } else {
        // With -- separator or explicit source: validate @targets
        let mut validated = Vec::new();
        for arg in &args.positionals {
            if let Some(stripped) = arg.strip_prefix('@') {
                if stripped.is_empty() {
                    eprintln!("Error: Empty target '@' is not allowed");
                    return 1;
                }
                validated.push(stripped.to_string());
            } else {
                let mut msg = format!("Error: Unexpected argument '{arg}'");
                if arg.chars().all(|c| c.is_alphabetic()) && arg.len() <= 20 {
                    msg.push_str(&format!("\nDid you mean @{arg}? Targets require @"));
                }
                eprintln!("{msg}");
                return 1;
            }
        }
        (validated, None)
    };

    // ── Resolve message ──
    let mut message = if let Some(msg) = compat_message {
        msg
    } else {
        match resolve_message(args) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("Error: {e}");
                return 1;
            }
        }
    };

    if let Err(err) = validate_message(&message) {
        eprintln!("Error: {err}");
        return 1;
    }

    // ── Resolve sender identity ──
    let sender_identity = if let Some(ref name) = from_name {
        SenderIdentity { kind: SenderKind::External, name: name.clone(), instance_data: None, session_id: None }
    } else if let Some(id) = ctx.and_then(|c| c.identity.as_ref()) {
        id.clone()
    } else if let Some(name) = explicit_name {
        match identity::resolve_identity(db, Some(name), None, None, None, None) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("Error: {e}");
                return 1;
            }
        }
    } else {
        match identity::resolve_identity(db, None, None, None, None, None) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("Error: {e}");
                return 1;
            }
        }
    };

    // Self-delivery is always filtered out, so an all-self target list would
    // otherwise "succeed" with no recipients.
    if !effective_targets.is_empty()
        && effective_targets.iter().all(|t| {
            // Routing treats bigboss:DEVICE as bigboss (device-agnostic identity).
            let base = match t.split_once(':') {
                Some((base, _)) if base == SENDER => base,
                _ => t.as_str(),
            };
            base == sender_identity.name
        })
    {
        eprintln!("Error: @{} is you ({}); agents don't receive their own messages", effective_targets[0], sender_identity.name);
        return 1;
    }

    let targets_to_pass: Option<&[String]> = if args.has_separator() || !effective_targets.is_empty() { Some(&effective_targets) } else { None };

    let preview_has_envelope = envelope.intent.is_some() || envelope.reply_to.is_some() || envelope.thread.is_some();
    let preview_delivery = match resolve_delivery(db, &sender_identity, &message, if preview_has_envelope { Some(&envelope) } else { None }, targets_to_pass) {
        Ok(delivery) => delivery,
        Err(e) => {
            eprintln!("Error: {e}");
            print_stopped_target_hints(db, &effective_targets, &message);
            return 1;
        }
    };

    if is_inside_ai_tool() && !ctx.map(|c| c.go).unwrap_or(false) && preview_delivery.original_scope == MessageScope::Broadcast && !preview_delivery.is_thread_resolved && preview_delivery.delivered_to.len() > 3 {
        print_broadcast_preview(db, &preview_delivery.delivered_to);
        return 1;
    }

    // ── Create bundle event if inline flags provided ──
    if let Some(mut bundle) = bundle_data {
        if let Err(e) = crate::messaging::bundles::validate_bundle(&mut bundle) {
            eprintln!("Error: {e}");
            return 1;
        }

        let bundle_instance = match sender_identity.kind {
            SenderKind::External => format!("ext_{}", sender_identity.name),
            SenderKind::System => format!("sys_{}", sender_identity.name),
            SenderKind::Instance => sender_identity.name.clone(),
        };

        match crate::messaging::bundles::create_bundle_event(&mut bundle, &bundle_instance, Some(&sender_identity.name), db) {
            Ok(bundle_id) => {
                crate::relay::worker::try_spawn_worker();
                envelope.bundle_id = Some(bundle_id.clone());

                // Append bundle summary text to message
                let refs = bundle.get("refs").cloned().unwrap_or(serde_json::json!({}));
                let events = refs.get("events").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().or(v.as_i64().map(|_| "")).or(Some(""))).map(|s| s.to_string()).collect::<Vec<_>>().join(", ")).unwrap_or_default();
                let files = refs.get("files").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default();
                let transcript = refs
                    .get("transcript")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| {
                                if let Some(obj) = v.as_object() { Some(format!("{}:{}", obj.get("range").and_then(|r| r.as_str()).unwrap_or(""), obj.get("detail").and_then(|d| d.as_str()).unwrap_or(""))) } else { v.as_str().map(|s| s.to_string()) }
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();

                let title = bundle.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let description = bundle.get("description").and_then(|v| v.as_str()).unwrap_or("");
                let extends = bundle.get("extends").and_then(|v| v.as_str());

                let mut bundle_lines =
                    vec![format!("[Bundle {bundle_id}]"), format!("Title: {title}"), format!("Description: {description}"), "Refs:".to_string(), format!("  events: {events}"), format!("  files: {files}"), format!("  transcript: {transcript}")];
                if let Some(ext) = extends {
                    bundle_lines.push(format!("Extends: {ext}"));
                }
                bundle_lines.push(String::new());
                bundle_lines.push("View bundle:".to_string());
                bundle_lines.push(format!("  agentx bundle cat {bundle_id}"));

                message = format!("{}\n\n{}", message.trim_end(), bundle_lines.join("\n"));
            }
            Err(e) => {
                eprintln!("Error: {e}");
                return 1;
            }
        }
    }

    // ── Send message ──
    let has_envelope = envelope.intent.is_some() || envelope.reply_to.is_some() || envelope.thread.is_some() || envelope.bundle_id.is_some();

    let (event_id, delivered_to) = match send_message(db, &sender_identity, &message, if has_envelope { Some(&envelope) } else { None }, targets_to_pass) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };
    // From here send owns the receiver's inline delivery: it delivers the batch
    // or deliberately leaves it unread (--quiet, --json, failed write). A send
    // that failed earlier leaves it to the router.
    crate::cli::context::claim_inline_delivery();

    // ── Feedback ──
    if args.json {
        let out = serde_json::json!({
            "event_id": event_id,
            "delivered_to": delivered_to,
        });
        println!("{}", serde_json::to_string(&out).unwrap());
        crate::relay::worker::try_spawn_worker();
        return 0;
    }

    if args.quiet {
        crate::relay::worker::try_spawn_worker();
        return 0;
    }

    let feedback = get_recipient_feedback(db, &delivered_to, envelope.thread.as_deref().filter(|_| preview_delivery.is_thread_resolved));

    // Use the same adhoc-only receive policy as other commands. With --from,
    // the invoking instance receives while the outgoing author stays external.
    let receiver = ctx.and_then(crate::cli::context::inline_receiver).map(|actor| (actor, !matches!(sender_identity.kind, SenderKind::Instance)));
    let batch = receiver.and_then(|(r, _)| InlineBatch::take(db, &r.name));
    let noted_remaining = batch.as_ref().is_some_and(|b| b.remaining > 0);
    if let (Some((receiver, set_status)), Some(batch)) = (receiver, batch) {
        let subagent_names: std::collections::HashSet<String> = db
            .conn()
            .prepare("SELECT name FROM instances WHERE parent_name = ?")
            .ok()
            .map(|mut stmt| stmt.query_map(rusqlite::params![&receiver.name], |row| row.get(0)).ok().into_iter().flatten().filter_map(|r| r.ok()).collect())
            .unwrap_or_default();
        let (sub_msgs, main_msgs): (Vec<_>, Vec<_>) = batch.messages.iter().partition(|msg| subagent_names.contains(&msg.from));

        let mut output = feedback.clone();
        if !main_msgs.is_empty() {
            output.push_str(&format!("\n{}", format_messages_for_hook(db, &main_msgs, &receiver.name)));
        }
        if !sub_msgs.is_empty() {
            output.push_str(&format!("\n[Subagent messages]\n{}", format_messages_for_hook(db, &sub_msgs, &receiver.name)));
        }
        output.push('\n');
        output.push_str(&batch.remaining_note(&receiver.name));
        if let Err(e) = batch.emit(db, &receiver.name, &output, set_status) {
            eprintln!("Message sent, but {e}");
            crate::relay::worker::try_spawn_worker();
            return 1;
        }
    } else {
        println!("{feedback}");
    }

    // Messages that arrived after the batch was taken are not in the output or
    // the remaining note. Say so, or an adhoc caller won't know until its next command.
    if let Some((receiver, _)) = receiver
        && !noted_remaining
        && !db.get_unread_messages(&receiver.name).is_empty()
    {
        println!("[agentx] new message(s) arrived — run: agentx listen --iam {}", receiver.name);
    }

    // Show intent tip
    if let Some(ref intent) = envelope.intent
        && matches!(sender_identity.kind, SenderKind::Instance)
    {
        let tip_key = format!("send:intent:{}", intent.as_str());
        crate::cli::tips::maybe_show_tip(db, &sender_identity.name, &tip_key, false);
    }

    crate::relay::worker::try_spawn_worker();

    0
}

/// Format messages for hook display (no ANSI).
#[cfg(test)]
fn format_messages_for_hook(_db: &AgentxDb, messages: &[&crate::db::Message], instance_name: &str) -> String {
    let recipient_display = instance_name;

    if messages.len() == 1 {
        let msg = messages[0];
        let sender_display = &msg.from;
        let prefix = crate::cli::context::format_envelope_prefix(msg.intent.as_deref(), msg.thread.as_deref(), msg.reply_id().as_deref());
        format!("{prefix} {sender_display} → {recipient_display}: {}", msg.text)
    } else {
        let parts: Vec<String> = messages
            .iter()
            .map(|msg| {
                let sender_display = &msg.from;
                let prefix = crate::cli::context::format_envelope_prefix(msg.intent.as_deref(), msg.thread.as_deref(), msg.reply_id().as_deref());
                format!("{prefix} {sender_display} → {recipient_display}: {}", msg.text)
            })
            .collect();
        format!("[{} new messages] | {}", parts.len(), parts.join(" | "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use serial_test::serial;
    use std::path::PathBuf;

    type TestEnv = (tempfile::TempDir, PathBuf, PathBuf, crate::integrations::hooks::test_helpers::EnvGuard);

    #[test]
    fn parse_basic_send() {
        let args = SendArgs::try_parse_from(["send", "@luna", "--", "hello", "there"]).unwrap();
        assert_eq!(args.positionals, vec!["@luna"]);
        assert_eq!(args.message, vec!["hello", "there"]);
    }

    #[test]
    fn parse_multiple_targets() {
        let args = SendArgs::try_parse_from(["send", "@luna", "@nova", "--", "hello"]).unwrap();
        assert_eq!(args.positionals, vec!["@luna", "@nova"]);
        assert_eq!(args.message, vec!["hello"]);
    }

    #[test]
    fn parse_broadcast() {
        let args = SendArgs::try_parse_from(["send", "--", "broadcast", "msg"]).unwrap();
        assert!(args.positionals.is_empty());
        assert_eq!(args.message, vec!["broadcast", "msg"]);
    }

    #[test]
    fn parse_with_intent_flag() {
        let args = SendArgs::try_parse_from(["send", "--intent", "request", "@luna", "--", "hello"]).unwrap();
        assert_eq!(args.intent.as_deref(), Some("request"));
        assert_eq!(args.positionals, vec!["@luna"]);
    }

    #[test]
    fn parse_flags_after_targets() {
        let args = SendArgs::try_parse_from(["send", "@luna", "--intent", "request", "--", "hello"]).unwrap();
        assert_eq!(args.intent.as_deref(), Some("request"));
        assert_eq!(args.positionals, vec!["@luna"]);
    }

    #[test]
    fn parse_bigboss_flag() {
        let args = SendArgs::try_parse_from(["send", "-b", "--", "hello"]).unwrap();
        assert!(args.bigboss);
        assert_eq!(args.sender_name(), Some("bigboss".to_string()));
    }

    #[test]
    fn parse_from_overrides_bigboss() {
        let args = SendArgs::try_parse_from(["send", "-b", "--from", "reviewer", "--", "hello"]).unwrap();
        assert_eq!(args.sender_name(), Some("reviewer".to_string()));
    }

    #[test]
    fn parse_stdin_flag() {
        let args = SendArgs::try_parse_from(["send", "--stdin", "@luna"]).unwrap();
        assert!(args.stdin);
        assert_eq!(args.positionals, vec!["@luna"]);
        assert!(args.message.is_empty());
    }

    #[test]
    fn parse_file_flag() {
        let args = SendArgs::try_parse_from(["send", "--file", "/tmp/msg.txt", "@luna"]).unwrap();
        assert_eq!(args.file.as_deref(), Some("/tmp/msg.txt"));
        assert_eq!(args.positionals, vec!["@luna"]);
    }

    #[test]
    fn parse_base64_flag() {
        let args = SendArgs::try_parse_from(["send", "--base64", "aGVsbG8=", "@luna"]).unwrap();
        assert_eq!(args.base64.as_deref(), Some("aGVsbG8="));
    }

    #[test]
    fn parse_reply_to_and_thread() {
        let args = SendArgs::try_parse_from(["send", "--reply-to", "42", "--thread", "pr-99", "@luna", "--", "hi"]).unwrap();
        assert_eq!(args.reply_to.as_deref(), Some("42"));
        assert_eq!(args.thread.as_deref(), Some("pr-99"));
    }

    #[test]
    fn parse_quiet_flag() {
        let args = SendArgs::try_parse_from(["send", "--quiet", "-b", "--", "hi"]).unwrap();
        assert!(args.quiet);
    }

    #[test]
    fn parse_inline_bundle_flags() {
        let args = SendArgs::try_parse_from(["send", "-b", "--title", "my-bundle", "--description", "desc", "--events", "1-10", "--files", "a.py", "--transcript", "1-5:normal", "--", "msg"]).unwrap();
        assert_eq!(args.title.as_deref(), Some("my-bundle"));
        assert_eq!(args.description.as_deref(), Some("desc"));
        assert_eq!(args.events.as_deref(), Some("1-10"));
        assert_eq!(args.files.as_deref(), Some("a.py"));
        assert_eq!(args.transcript.as_deref(), Some("1-5:normal"));
    }

    #[test]
    fn parse_no_separator_targets_only() {
        let args = SendArgs::try_parse_from(["send", "@luna"]).unwrap();
        assert_eq!(args.positionals, vec!["@luna"]);
        assert!(args.message.is_empty());
    }

    #[test]
    fn parse_compat_at_name_with_space() {
        // Backward compat: '@luna hi' as a single quoted arg
        // process_positionals treats as full message text
        let args = SendArgs::try_parse_from(["send", "@luna hi"]).unwrap();
        assert_eq!(args.positionals, vec!["@luna hi"]);
    }

    #[test]
    fn parse_bare_text_accepted() {
        // Bare text without @ is accepted by clap; cmd_send handles as message
        let args = SendArgs::try_parse_from(["send", "hello everyone"]).unwrap();
        assert_eq!(args.positionals, vec!["hello everyone"]);
    }

    #[test]
    fn parse_empty_target_rejected() {
        let result = SendArgs::try_parse_from(["send", "@", "--", "hi"]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_bare_text_with_separator_accepted() {
        // Bare text before -- is accepted by clap; validated in cmd_send
        let args = SendArgs::try_parse_from(["send", "luna", "--", "hi"]).unwrap();
        assert_eq!(args.positionals, vec!["luna"]);
        assert_eq!(args.message, vec!["hi"]);
    }

    #[test]
    fn parse_message_with_dashes() {
        let args = SendArgs::try_parse_from(["send", "@luna", "--", "--this", "is", "a", "message"]).unwrap();
        assert_eq!(args.message, vec!["--this", "is", "a", "message"]);
    }

    #[test]
    fn resolve_message_keeps_trailing_identity_tokens() {
        let mut args = SendArgs::try_parse_from(["send", "@luna", "--", "message", "--iam", "beru"]).unwrap();
        args.had_separator = true;

        assert_eq!(resolve_message(&args).unwrap(), "message --iam beru");
    }

    #[test]
    fn resolve_message_keeps_quoted_name_suffix_inside_one_argument() {
        let mut args = SendArgs::try_parse_from(["send", "@luna", "--", "I always end commands with --iam beru"]).unwrap();
        args.had_separator = true;

        assert_eq!(resolve_message(&args).unwrap(), "I always end commands with --iam beru");
    }

    #[test]
    fn resolve_message_keeps_trailing_name_for_different_sender() {
        let mut args = SendArgs::try_parse_from(["send", "@luna", "--", "message", "--iam", "nova"]).unwrap();
        args.had_separator = true;

        assert_eq!(resolve_message(&args).unwrap(), "message --iam nova");
    }

    #[test]
    fn parse_extends_flag() {
        let args = SendArgs::try_parse_from(["send", "-b", "--title", "t", "--extends", "abc123", "--", "msg"]).unwrap();
        assert_eq!(args.extends.as_deref(), Some("abc123"));
    }

    // ── process_positionals tests ──

    #[test]
    fn process_empty() {
        let (targets, msg) = process_positionals(&[]);
        assert!(targets.is_empty());
        assert!(msg.is_none());
    }

    fn setup_test_db() -> (AgentxDb, PathBuf, TestEnv) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        // send_message() reaches the process-global relay notification path,
        // so its ambient AGENTX_DIR must live as long as the test DB.
        let env = crate::integrations::hooks::test_helpers::isolated_test_env();
        let temp_dir = std::env::temp_dir();
        let test_id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let db_path = temp_dir.join(format!("test_agentx_send_{}_{}.db", std::process::id(), test_id));

        let db = AgentxDb::open_at(&db_path).unwrap();
        (db, db_path, env)
    }

    fn cleanup_test_db(path: PathBuf) {
        let _ = std::fs::remove_file(&path);
        let wal = PathBuf::from(format!("{}-wal", path.display()));
        let shm = PathBuf::from(format!("{}-shm", path.display()));
        let _ = std::fs::remove_file(wal);
        let _ = std::fs::remove_file(shm);
    }

    #[test]
    #[serial]
    fn send_message_threads_seed_and_reuse_memberships() {
        let (db, path, _env) = setup_test_db();
        db.conn().execute("INSERT INTO instances (name, created_at) VALUES ('luna', 1000.0), ('nova', 1000.0), ('miso', 1000.0)", []).unwrap();

        let sender = SenderIdentity { kind: SenderKind::Instance, name: "luna".into(), instance_data: None, session_id: None };
        let envelope = MessageEnvelope { thread: Some("debate-1".into()), ..Default::default() };

        let (_, delivered) = send_message(&db, &sender, "hello", Some(&envelope), Some(&["nova".to_string(), "miso".to_string()])).unwrap();
        assert_eq!(delivered, vec!["nova".to_string(), "miso".to_string()]);

        let members = db.get_thread_members("debate-1");
        assert_eq!(members, vec!["nova".to_string(), "miso".to_string(), "luna".to_string()]);

        let (_, delivered) = send_message(&db, &sender, "round 2", Some(&envelope), None).unwrap();
        assert_eq!(delivered, vec!["nova".to_string(), "miso".to_string()]);

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn send_message_thread_without_members_errors() {
        let (db, path, _env) = setup_test_db();
        db.conn().execute("INSERT INTO instances (name, created_at) VALUES ('luna', 1000.0)", []).unwrap();

        let sender = SenderIdentity { kind: SenderKind::Instance, name: "luna".into(), instance_data: None, session_id: None };
        let envelope = MessageEnvelope { thread: Some("empty-thread".into()), ..Default::default() };

        let err = send_message(&db, &sender, "hello", Some(&envelope), None).unwrap_err();
        assert!(err.contains("has no members"));

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn send_message_external_sender_does_not_auto_subscribe_to_thread() {
        let (db, path, _env) = setup_test_db();
        db.conn().execute("INSERT INTO instances (name, created_at) VALUES ('nova', 1000.0)", []).unwrap();

        let sender = SenderIdentity { kind: SenderKind::External, name: "bigboss".into(), instance_data: None, session_id: None };
        let envelope = MessageEnvelope { thread: Some("ops".into()), ..Default::default() };

        let (_, delivered) = send_message(&db, &sender, "hello", Some(&envelope), Some(&["nova".to_string()])).unwrap();
        assert_eq!(delivered, vec!["nova".to_string()]);
        assert_eq!(db.get_thread_members("ops"), vec!["nova".to_string()]);

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn send_message_thread_request_does_not_create_request_watch_rows() {
        let (db, path, _env) = setup_test_db();
        db.conn().execute("INSERT INTO instances (name, created_at) VALUES ('luna', 1000.0), ('nova', 1000.0)", []).unwrap();

        let sender = SenderIdentity { kind: SenderKind::Instance, name: "luna".into(), instance_data: None, session_id: None };
        let seed_envelope = MessageEnvelope { thread: Some("ops".into()), ..Default::default() };
        send_message(&db, &sender, "seed", Some(&seed_envelope), Some(&["nova".to_string()])).unwrap();

        let request_envelope = MessageEnvelope { intent: Some(crate::messaging::routing::MessageIntent::Request), thread: Some("ops".into()), ..Default::default() };
        let (_, delivered) = send_message(&db, &sender, "status?", Some(&request_envelope), None).unwrap();
        assert_eq!(delivered, vec!["nova".to_string()]);

        let reqwatch_count: i64 = db.conn().query_row("SELECT COUNT(*) FROM kv WHERE key LIKE 'events_sub:reqwatch-%'", [], |row| row.get(0)).unwrap();
        assert_eq!(reqwatch_count, 0);

        let (scope, mentions_json): (String, String) = db
            .conn()
            .query_row(
                "SELECT json_extract(data, '$.scope'), json_extract(data, '$.mentions')
                 FROM events
                 WHERE type = 'message'
                 ORDER BY id DESC
                 LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(scope, "mentions");
        assert!(mentions_json.contains("nova"));

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn send_mention_excludes_inactive_instances() {
        let (db, path, _env) = setup_test_db();
        db.conn()
            .execute(
                "INSERT INTO instances (name, status, status_context, created_at)
                 VALUES ('luna', 'listening', '', 1000.0),
                        ('vine', 'inactive', 'exit:unknown', 1000.0)",
                [],
            )
            .unwrap();

        let sender = SenderIdentity { kind: SenderKind::Instance, name: "luna".into(), instance_data: None, session_id: None };

        let err = send_message(&db, &sender, "ping", None, Some(&["vine".to_string()])).unwrap_err();
        assert!(err.contains("@vine"), "err={err}");
        assert!(err.contains("Available:"), "err={err}");
        assert!(err.contains("luna"), "listening agent should be listed: {err}");
        let available_line = err.lines().find(|l| l.contains("Available:")).unwrap_or("");
        assert!(!available_line.contains("vine"), "inactive agent must not appear in Available: {err}");

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn send_mention_excludes_exit_no_tool_call_inactive() {
        let (db, path, _env) = setup_test_db();
        db.conn()
            .execute(
                "INSERT INTO instances (name, status, status_context, created_at)
                 VALUES ('luna', 'listening', '', 1000.0),
                        ('dove', 'inactive', 'exit:no_tool_call', 1000.0)",
                [],
            )
            .unwrap();

        let sender = SenderIdentity { kind: SenderKind::Instance, name: "luna".into(), instance_data: None, session_id: None };

        let err = send_message(&db, &sender, "ping", None, Some(&["dove".to_string()])).unwrap_err();
        assert!(err.contains("@dove"), "err={err}");
        let available_line = err.lines().find(|l| l.contains("Available:")).unwrap_or("");
        assert!(!available_line.contains("dove"), "soft-stopped agent must not appear in Available: {err}");

        cleanup_test_db(path);
    }

    #[test]
    fn process_compat_at_with_space() {
        // "@luna hi" → full text as message, no targets
        let (targets, msg) = process_positionals(&["@luna hi".to_string()]);
        assert!(targets.is_empty());
        assert_eq!(msg.as_deref(), Some("@luna hi"));
    }

    #[test]
    fn process_bare_text() {
        // "hello everyone" → message text, no targets (broadcast)
        let (targets, msg) = process_positionals(&["hello everyone".to_string()]);
        assert!(targets.is_empty());
        assert_eq!(msg.as_deref(), Some("hello everyone"));
    }

    #[test]
    fn process_pure_targets() {
        // "@luna" → target, no message
        let (targets, msg) = process_positionals(&["@luna".to_string()]);
        assert_eq!(targets, vec!["luna"]);
        assert!(msg.is_none());
    }

    #[test]
    fn process_target_plus_bare_text() {
        // "@luna", "hello" → target + message
        let (targets, msg) = process_positionals(&["@luna".to_string(), "hello".to_string()]);
        assert_eq!(targets, vec!["luna"]);
        assert_eq!(msg.as_deref(), Some("hello"));
    }

    fn insert_imported_remote_message(db: &AgentxDb, local_id: i64, origin_id: serde_json::Value, short: &str, text: &str, thread: Option<&str>, intent: Option<&str>) {
        let relay_obj = serde_json::json!({
            "id": origin_id,
            "short": short,
            "device": format!("dev-uuid-{short}"),
        });
        let mut data = serde_json::json!({
            "from": format!("remote:{short}"),
            "text": text,
            "_relay": relay_obj,
        });
        if let Some(t) = thread {
            data["thread"] = serde_json::json!(t);
        }
        if let Some(i) = intent {
            data["intent"] = serde_json::json!(i);
        }
        db.conn().execute("INSERT INTO events (id, type, instance, timestamp, data) VALUES (?, 'message', 'remote-inst', '2026-09-04T00:00:00Z', ?)", rusqlite::params![local_id, data.to_string()]).unwrap();
    }

    #[test]
    #[serial]
    fn test_resolve_reply_to_remote_different_local_id() {
        let (db, path, _env) = setup_test_db();
        insert_imported_remote_message(&db, 100, serde_json::json!(42), "BOXE", "remote msg", None, None);

        let resolved = resolve_reply_to_local(&db, "42:BOXE");
        assert_eq!(resolved, Some(100));

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn test_resolve_reply_to_remote_colliding_local_id() {
        let (db, path, _env) = setup_test_db();
        let local_data = serde_json::json!({ "text": "unrelated local message" });
        db.conn().execute("INSERT INTO events (id, type, instance, timestamp, data) VALUES (42, 'message', 'local-inst', '2026-09-04T00:00:00Z', ?)", [local_data.to_string()]).unwrap();

        insert_imported_remote_message(&db, 100, serde_json::json!(42), "BOXE", "remote msg", None, None);

        // 42:BOXE must resolve to the remote event (100), never the colliding local event (42)
        assert_eq!(resolve_reply_to_local(&db, "42:BOXE"), Some(100));
        // local 42 without suffix must resolve to the local event (42)
        assert_eq!(resolve_reply_to_local(&db, "42"), Some(42));

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn test_resolve_reply_to_two_devices_same_origin_id() {
        let (db, path, _env) = setup_test_db();
        insert_imported_remote_message(&db, 100, serde_json::json!(42), "BOXE", "msg 1", None, None);
        insert_imported_remote_message(&db, 200, serde_json::json!(42), "WAVE", "msg 2", None, None);

        assert_eq!(resolve_reply_to_local(&db, "42:BOXE"), Some(100));
        assert_eq!(resolve_reply_to_local(&db, "42:WAVE"), Some(200));

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn test_resolve_reply_to_canonical_device_suffix_grammar() {
        let (db, path, _env) = setup_test_db();
        insert_imported_remote_message(&db, 100, serde_json::json!(42), "BOXE", "remote msg", None, None);

        // Exact 4-char uppercase ASCII succeeds
        assert_eq!(resolve_reply_to_local(&db, "42:BOXE"), Some(100));

        // Lowercase, mixed-case, prefixes, or non-4-char suffixes must fail closed
        assert_eq!(resolve_reply_to_local(&db, "42:boxe"), None);
        assert_eq!(resolve_reply_to_local(&db, "42:Boxe"), None);
        assert_eq!(resolve_reply_to_local(&db, "42:BO"), None);
        assert_eq!(resolve_reply_to_local(&db, "42:BOX"), None);
        assert_eq!(resolve_reply_to_local(&db, "42:BOXES"), None);
        assert_eq!(resolve_reply_to_local(&db, "42:NOPE"), None);

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn test_resolve_reply_to_rejects_non_integer_json_id() {
        let (db, path, _env) = setup_test_db();
        // Origin ID stored as JSON string instead of JSON integer
        insert_imported_remote_message(&db, 100, serde_json::json!("42"), "BOXE", "malformed", None, None);

        // Must fail closed because json_type is not 'integer'
        assert_eq!(resolve_reply_to_local(&db, "42:BOXE"), None);

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn test_resolve_reply_to_duplicate_same_short_origin_fails_closed() {
        let (db, path, _env) = setup_test_db();
        insert_imported_remote_message(&db, 100, serde_json::json!(42), "BOXE", "msg 1", None, None);
        insert_imported_remote_message(&db, 101, serde_json::json!(42), "BOXE", "msg 2", None, None);

        // Multiple rows with origin=42 and short=BOXE must fail closed
        assert_eq!(resolve_reply_to_local(&db, "42:BOXE"), None);

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn test_cmd_send_remote_reply_inherits_thread_at_command_boundary() {
        let (db, path, _env) = setup_test_db();
        db.conn().execute("INSERT INTO instances (name, created_at) VALUES ('sender', 1000.0), ('target', 1000.0)", []).unwrap();

        // Remote request at local row 100 with origin 42 and thread "thread-remote-99"
        insert_imported_remote_message(&db, 100, serde_json::json!(42), "BOXE", "remote req", Some("thread-remote-99"), Some("request"));

        // Execute send through the full cmd_send entrypoint without specifying --thread
        let mut args = SendArgs::try_parse_from(["send", "@target", "--from", "sender", "--intent", "ack", "--reply-to", "42:BOXE", "--", "pong message"]).unwrap();
        args.had_separator = true;

        let exit_code = cmd_send(&db, &args, None);
        assert_eq!(exit_code, 0, "cmd_send should succeed");

        // Verify sent event data at command boundary
        let (reply_to, reply_to_local, thread): (Option<String>, Option<i64>, Option<String>) = db
            .conn()
            .query_row(
                "SELECT json_extract(data, '$.reply_to'), json_extract(data, '$.reply_to_local'), json_extract(data, '$.thread')
                 FROM events WHERE type = 'message' AND id != 100 ORDER BY id DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();

        assert_eq!(reply_to.as_deref(), Some("42:BOXE"));
        assert_eq!(reply_to_local, Some(100));
        assert_eq!(thread.as_deref(), Some("thread-remote-99"));

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn test_cmd_send_unresolved_remote_reply_exits_one() {
        let (db, path, _env) = setup_test_db();
        db.conn().execute("INSERT INTO instances (name, created_at) VALUES ('sender', 1000.0), ('target', 1000.0)", []).unwrap();

        insert_imported_remote_message(&db, 100, serde_json::json!(42), "BOXE", "remote req", None, None);

        let count_before: i64 = db.conn().query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0)).unwrap();

        // Unknown device
        let mut args_unknown = SendArgs::try_parse_from(["send", "@target", "--from", "sender", "--reply-to", "42:NOPE", "--", "hi"]).unwrap();
        args_unknown.had_separator = true;
        assert_eq!(cmd_send(&db, &args_unknown, None), 1);

        // Non-canonical lowercase device
        let mut args_lower = SendArgs::try_parse_from(["send", "@target", "--from", "sender", "--reply-to", "42:boxe", "--", "hi"]).unwrap();
        args_lower.had_separator = true;
        assert_eq!(cmd_send(&db, &args_lower, None), 1);

        // Non-existent origin ID
        let mut args_nonexistent = SendArgs::try_parse_from(["send", "@target", "--from", "sender", "--reply-to", "999:BOXE", "--", "hi"]).unwrap();
        args_nonexistent.had_separator = true;
        assert_eq!(cmd_send(&db, &args_nonexistent, None), 1);

        let count_after: i64 = db.conn().query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0)).unwrap();
        assert_eq!(count_before, count_after);

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn test_cmd_send_ambiguous_remote_reply_exits_one_and_emits_no_message() {
        let (db, path, _env) = setup_test_db();
        db.conn().execute("INSERT INTO instances (name, created_at) VALUES ('sender', 1000.0), ('target', 1000.0)", []).unwrap();

        // Two distinct local rows with identical origin ID (42) and short device ("BOXE")
        insert_imported_remote_message(&db, 101, serde_json::json!(42), "BOXE", "remote msg 1", None, None);
        insert_imported_remote_message(&db, 102, serde_json::json!(42), "BOXE", "remote msg 2", None, None);

        let count_before: i64 = db.conn().query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0)).unwrap();

        let mut args_ambiguous = SendArgs::try_parse_from(["send", "@target", "--from", "sender", "--reply-to", "42:BOXE", "--", "hi"]).unwrap();
        args_ambiguous.had_separator = true;

        assert_eq!(cmd_send(&db, &args_ambiguous, None), 1);

        let count_after: i64 = db.conn().query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0)).unwrap();
        assert_eq!(count_before, count_after);

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn test_send_message_remote_ack_loop_prevention_with_differing_ids() {
        let (db, path, _env) = setup_test_db();
        db.conn().execute("INSERT INTO instances (name, created_at) VALUES ('luna', 1000.0)", []).unwrap();

        // 1. Remote inform: local id = 110, origin id = 10 (distinct IDs)
        insert_imported_remote_message(&db, 110, serde_json::json!(10), "BOXE", "remote info", None, Some("inform"));

        let sender = SenderIdentity { kind: SenderKind::Instance, name: "luna".into(), instance_data: None, session_id: None };

        let ack_to_inform = MessageEnvelope { intent: Some(crate::messaging::routing::MessageIntent::Ack), reply_to: Some("10:BOXE".into()), ..Default::default() };

        let err = send_message(&db, &sender, "ack", Some(&ack_to_inform), None).unwrap_err();
        assert!(err.contains("Cannot ack an inform"));

        // 2. Remote ack: local id = 120, origin id = 20 (distinct IDs)
        insert_imported_remote_message(&db, 120, serde_json::json!(20), "BOXE", "remote ack", None, Some("ack"));

        let ack_to_ack = MessageEnvelope { intent: Some(crate::messaging::routing::MessageIntent::Ack), reply_to: Some("20:BOXE".into()), ..Default::default() };

        let err = send_message(&db, &sender, "ack", Some(&ack_to_ack), None).unwrap_err();
        assert!(err.contains("Ack-on-ack loop detected"));

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn test_resolve_reply_to_malformed_tokens_fail_closed() {
        let (db, path, _env) = setup_test_db();
        insert_imported_remote_message(&db, 100, serde_json::json!(42), "BOXE", "remote msg", None, None);

        assert_eq!(resolve_reply_to_local(&db, ""), None);
        assert_eq!(resolve_reply_to_local(&db, "   "), None);
        assert_eq!(resolve_reply_to_local(&db, "42:"), None);
        assert_eq!(resolve_reply_to_local(&db, ":BOXE"), None);
        assert_eq!(resolve_reply_to_local(&db, "42:BOXE:EXTRA"), None);
        assert_eq!(resolve_reply_to_local(&db, "abc:BOXE"), None);

        cleanup_test_db(path);
    }

    #[test]
    #[serial]
    fn test_resolve_reply_to_local_only_unchanged() {
        let (db, path, _env) = setup_test_db();
        let local_data = serde_json::json!({ "text": "local message" });
        db.conn().execute("INSERT INTO events (id, type, instance, timestamp, data) VALUES (42, 'message', 'local-inst', '2026-09-04T00:00:00Z', ?)", [local_data.to_string()]).unwrap();

        assert_eq!(resolve_reply_to_local(&db, "42"), Some(42));
        assert_eq!(resolve_reply_to_local(&db, "999"), None);
        assert_eq!(resolve_reply_to_local(&db, "notanumber"), None);

        cleanup_test_db(path);
    }
}
