//! The finite cloud messaging vocabulary shared by the direct client and
//! Linux daemon IPC. Only these commands can construct an upstream request.

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::{Actor, http::Http, path_segment, valid_name};
use crate::cloud::communication::types::{AgentRegistration, ClaimInbox, ConfirmInbox, Descriptor, RecoverInbox, SendMessage};
#[cfg(target_os = "linux")]
use crate::cloud::communication::types::{AgentView, HostRegistration, HostView};

pub(super) enum Port {
    Direct(Http),
    #[cfg(target_os = "linux")]
    Managed(super::ipc::Client),
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) enum Selection {
    Bound(String),
    Operator(Option<String>),
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "op", content = "args", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Command {
    Info,
    Agents(Option<String>),
    Hosts(Option<String>),
    Resolve(Selection),
    Send(SendMessage),
    Receipt { request_id: String, actor: Option<Actor> },
    Claim(ClaimInbox),
    Recover(RecoverInbox),
    Confirm(ConfirmInbox),
    Register { actor: Actor, daemon_instance: Option<String> },
    Retire(Actor),
}

impl Port {
    pub(super) fn call<T: DeserializeOwned>(&self, command: Command) -> Result<T> {
        match self {
            Self::Direct(http) => Ok(serde_json::from_value(command.upstream(http)?)?),
            #[cfg(target_os = "linux")]
            Self::Managed(client) => client.call(command),
        }
    }
}

pub(super) fn checked_descriptor(http: &Http, url: &str) -> Result<Descriptor> {
    let descriptor: Descriptor = http.request("GET", "/descriptor", None)?;
    ensure!(descriptor.protocol_version == 1, "unsupported cloud scope protocol version {}", descriptor.protocol_version);
    ensure!(url.rsplit_once('/').is_some_and(|(_, id)| id == descriptor.scope_id), "cloud descriptor does not match the selected scope URL");
    Ok(descriptor)
}

#[cfg(target_os = "linux")]
pub(super) fn heartbeat(http: &Http, actor: &Actor) -> Result<()> {
    request::<HostView>(http, "PUT", &format!("/hosts/{}", path_segment(&actor.agent.host)), &HostRegistration { generation: actor.generation.clone(), installation_id: actor.installation_id.clone() })?;
    Ok(())
}

fn request<T: DeserializeOwned>(http: &Http, method: &str, path: &str, body: &impl Serialize) -> Result<T> {
    http.request(method, path, Some(&serde_json::to_value(body)?))
}

fn page(which: &str, after: Option<&str>) -> String {
    match after {
        Some(cursor) => format!("/{which}?limit=200&after={}", path_segment(cursor)),
        None => format!("/{which}?limit=200"),
    }
}

impl Command {
    pub(super) fn upstream(self, http: &Http) -> Result<Value> {
        let registering = matches!(self, Self::Register { .. });
        match self {
            Self::Info | Self::Resolve(_) => anyhow::bail!("local IPC operation has no upstream route"),
            Self::Agents(after) => http.request("GET", &page("agents", after.as_deref()), None),
            Self::Hosts(after) => http.request("GET", &page("hosts", after.as_deref()), None),
            Self::Send(body) => request(http, "POST", "/messages", &body),
            Self::Receipt { request_id, actor } => {
                let receipt: super::MessageOperation = http.request("GET", &format!("/operations/{}", path_segment(&request_id)), None)?;
                ensure!(actor.is_none_or(|binding| receipt.message.from == binding.agent), "message receipt belongs to another Agent");
                Ok(serde_json::to_value(receipt)?)
            }
            Self::Claim(body) => request(http, "POST", "/inbox/claim", &body),
            Self::Recover(body) => request(http, "POST", "/inbox/recover", &body),
            Self::Confirm(body) => request(http, "POST", "/inbox/confirm", &body),
            Self::Register { actor, .. } | Self::Retire(actor) => {
                valid_name(&actor.agent.name)?;
                let body = AgentRegistration { generation: actor.generation, installation_id: actor.installation_id, host: actor.agent.host, incarnation: actor.agent.incarnation };
                let name = path_segment(&actor.agent.name);
                if registering { request(http, "PUT", &format!("/agents/{name}"), &body) } else { request(http, "POST", &format!("/agents/{name}/retire"), &body) }
            }
        }
    }
}

#[cfg(target_os = "linux")]
impl Command {
    pub(super) fn actor(&self) -> Option<(&str, &str, &AgentView)> {
        Some(match self {
            Self::Send(b) => (&b.generation, &b.installation_id, &b.from),
            Self::Claim(b) => (&b.generation, &b.installation_id, &b.agent),
            Self::Recover(b) => (&b.generation, &b.installation_id, &b.agent),
            Self::Confirm(b) => (&b.generation, &b.installation_id, &b.agent),
            Self::Register { actor: a, .. } | Self::Retire(a) | Self::Receipt { actor: Some(a), .. } => (&a.generation, &a.installation_id, &a.agent),
            _ => return None,
        })
    }
}
