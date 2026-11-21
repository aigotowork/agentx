//! Hermes gateway command adapter. Typed registry operations return before
//! Provider configuration, file refresh, endpoint I/O or bootstrap rendering.
use crate::agents::hermes_service::{self as registry, ActiveBridge, RegistryEntry};
use crate::integrations::hooks::plugin_rpc::{self, parse_flag};
use crate::runtime::context::AgentxContext;
use crate::runtime::hosts::{HostService, Request as HostRequest};
use crate::service::client::Client;
use crate::service::{Request, Response};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub(super) fn connect(context: &AgentxContext) -> Result<Client> {
    Ok(Client::connect(&context.agentx_dir, crate::runtime::host_client::invocation(context))?)
}
pub(super) fn call(client: &Client, request: registry::Request) -> Result<registry::Response> {
    match client.call(Request::Participants(crate::agents::service::Request::Hermes(request)))? {
        Response::Participants(crate::agents::service::Response::Hermes(response)) => Ok(response),
        _ => bail!("unexpected Hermes registry response"),
    }
}
fn home(context: &AgentxContext, raw: &str) -> Result<PathBuf> {
    if raw.is_empty() {
        bail!("Hermes profile required");
    }
    let path = Path::new(raw);
    let path = if path.is_absolute() { path.to_path_buf() } else { context.cwd.join(path) };
    // Originating host owns filesystem normalization. No daemon filesystem
    // state or cwd chooses the profile on another caller's behalf.
    path.canonicalize().with_context(|| format!("cannot resolve Hermes profile {}", path.display()))
}
pub(super) fn list(client: &Client, home: Option<PathBuf>) -> Result<Vec<RegistryEntry>> {
    match call(client, registry::Request::List { home })? {
        registry::Response::Registry(entries) => Ok(entries),
        _ => bail!("unexpected Hermes registry listing"),
    }
}
fn actor(client: &Client, argv: &[String]) -> Result<crate::agents::service::ResolvedActor> {
    let name = parse_flag(argv, "--name").context("Missing --name")?;
    match client.call(Request::Participants(crate::agents::service::Request::Resolve))? {
        Response::Participants(crate::agents::service::Response::Actor(actor)) if actor.record.tool == "hermes" && actor.participant.canonical_name == name => Ok(actor),
        _ => bail!("{name} is not attached to this Hermes process"),
    }
}
fn bridge_client(client: &Client, context: &AgentxContext, entry: &ActiveBridge) -> (Client, AgentxContext) {
    let mut environment = context.raw_env.clone();
    environment.insert("AGENTX_PROCESS_ID".into(), entry.bridge.token.clone());
    environment.insert("AGENTX_LAUNCHED".into(), "1".into());
    environment.insert("AGENTX_TOOL".into(), "hermes".into());
    environment.insert("AGENTX_DIR".into(), client.scope_root().to_string_lossy().into_owned());
    let mut context = AgentxContext::from_env(&environment, PathBuf::from(&entry.bridge.home));
    context.agentx_dir = client.scope_root().to_path_buf();
    let client = client.with_invocation(crate::runtime::host_client::invocation(&context));
    (client, context)
}
fn entry(client: &Client, context: &AgentxContext, active: ActiveBridge, port: Option<u16>) -> Result<Value> {
    let (client, context) = bridge_client(client, context, &active);
    if let Some(port) = port {
        if port == 0 {
            bail!("Hermes notify port must be nonzero");
        }
        HostService::call(&client, HostRequest::SetPluginEndpoint { participant: active.participant.clone(), port, standby: false })?;
    }
    let mut bootstrap = crate::runtime::bootstrap::get_instructions_client(&client, &context, &active.bridge.name, "hermes")?;
    let command = super::bridge::agentx_command_with_context(&context);
    if !bootstrap.is_empty() && command != "agentx" {
        bootstrap.push_str(&format!("\n\n[AGENTX BRIDGE]\nRun agentx here as `{command}` (the plain `agentx` on PATH talks to another agentx)."));
    }
    Ok(json!({"name":active.bridge.name,"token":active.bridge.token,"session_key":active.bridge.session_key,"origin":active.bridge.origin,"visibility":active.bridge.visibility,"bootstrap":bootstrap,"recreated":active.recreated}))
}
fn dispatch_inner(client: &Client, context: &AgentxContext, sub: &str, argv: &[String]) -> Result<Value> {
    match sub {
        "add" => {
            let home = home(context, &parse_flag(argv, "--home").context("Missing --home")?)?;
            let session_key = parse_flag(argv, "--session-key").context("Missing --session-key")?;
            let platform = parse_flag(argv, "--platform").context("Missing --platform")?;
            let name = parse_flag(argv, "--name").filter(|name| !name.is_empty());
            if session_key.is_empty() || platform.is_empty() {
                bail!("Hermes chat and platform required");
            }
            if let Some(name) = &name
                && !crate::agents::identity::is_valid_name(name)
            {
                bail!("{}", crate::agents::identity::name_error(name));
            }
            let registered = list(client, Some(home.clone()))?;
            if !registered.iter().any(|entry| entry.bridge.session_key == session_key) {
                super::grant_injection_with_client(client, context, &home)?;
            }
            let request = registry::Add {
                home,
                session_key,
                platform,
                chat: parse_flag(argv, "--chat").unwrap_or_default(),
                origin: parse_flag(argv, "--origin").and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or(Value::Null),
                name,
                session_id: parse_flag(argv, "--session-id").filter(|session| !session.is_empty()),
            };
            match call(client, registry::Request::Add(request))? {
                registry::Response::Active(active) => entry(client, context, active, None),
                _ => bail!("unexpected Hermes registration response"),
            }
        }
        "sync" => {
            let home = home(context, &parse_flag(argv, "--home").context("Missing --home")?)?;
            if argv.iter().any(|arg| arg == "--ensure-grant") && !list(client, Some(home.clone()))?.is_empty() {
                super::grant_injection_with_client(client, context, &home)?;
            }
            let loaded = parse_flag(argv, "--plugin-hash");
            let current = loaded.as_deref() == Some(super::plugin_hash().as_str());
            let reload = super::refresh_gateway_code(&home, loaded.as_deref());
            call(client, registry::Request::GatewayVersion { home: home.clone(), current })?;
            let port = parse_flag(argv, "--notify-port").map(|port| port.parse::<u16>()).transpose()?;
            let active = match call(client, registry::Request::Sync { home })? {
                registry::Response::Synced(active) => active,
                _ => bail!("unexpected Hermes synchronization response"),
            };
            let entries = active.into_iter().map(|active| entry(client, context, active, port)).collect::<Result<Vec<_>>>()?;
            Ok(json!({"bridges":entries,"reload_plugin":reload}))
        }
        "session" => {
            let actor = actor(client, argv)?;
            let session_id = parse_flag(argv, "--session-id").context("Missing --session-id")?;
            call(client, registry::Request::BindSession { participant: actor.participant, session_id })?;
            Ok(json!({"ok":true}))
        }
        "visibility" => {
            let actor = actor(client, argv)?;
            let visibility = match parse_flag(argv, "--mode").as_deref() {
                Some("off") => registry::Visibility::Off,
                Some("preview") => registry::Visibility::Preview,
                Some("full") => registry::Visibility::Full,
                _ => bail!("Visibility must be off, preview or full"),
            };
            match call(client, registry::Request::Visibility { participant: actor.participant, visibility })? {
                registry::Response::Active(active) => entry(client, context, active, None),
                _ => bail!("unexpected Hermes visibility response"),
            }
        }
        "rm" => {
            let name = parse_flag(argv, "--name").context("Missing --name")?;
            let expected = list(client, None)?.into_iter().find(|entry| entry.bridge.name == name).with_context(|| format!("{name} is not a bridge"))?;
            match call(client, registry::Request::Remove { expected })? {
                registry::Response::Removed { effects, .. } => {
                    for effect in effects {
                        crate::runtime::stop_effects::apply(effect);
                    }
                }
                _ => bail!("unexpected Hermes disconnect response"),
            }
            Ok(json!({"ok":true}))
        }
        _ => bail!("Unknown hermes-bridge command: {sub}"),
    }
}
pub(super) fn dispatch(argv: &[String]) -> (i32, String) {
    let context = AgentxContext::from_os();
    let args = argv.get(1..).unwrap_or_default();
    let (sub, rest) = args.split_first().map_or(("", &[][..]), |(sub, args)| (sub.as_str(), args));
    let result = crate::integrations::hooks::common::dispatch_with_panic_guard("hermes", "hermes-bridge", Err(anyhow::anyhow!("internal panic")), || {
        let client = connect(&context)?;
        dispatch_inner(&client, &context, sub, rest)
    });
    match result {
        Ok(output) => (0, output.to_string()),
        Err(error) => plugin_rpc::error(format!("{error:#}")),
    }
}
