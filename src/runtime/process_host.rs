//! Client-side PTY process registration. The effective invocation is captured
//! at the host, while daemon persistence returns a participant/process fence.

use anyhow::{Result, bail};

use crate::agents::service::{Request as ParticipantRequest, Response as ParticipantResponse};
use crate::runtime::context::AgentxContext;
use crate::service::client::Client;
use crate::service::{Request as ServiceRequest, Response as ServiceResponse};

use super::hosts::{ChildProcess, EndpointKind, HostService, ProcessRegistration, Request, Response};

#[derive(Clone)]
pub(crate) struct ProcessHost {
    client: Client,
    registration: ProcessRegistration,
    pub display_name: String,
    endpoints: std::sync::Arc<std::sync::Mutex<Vec<(EndpointKind, u16)>>>,
}

fn connect(context: &AgentxContext, name: &str) -> Result<(Client, crate::agents::service::ResolvedActor)> {
    let mut invocation = super::host_client::invocation(context);
    if invocation.process_binding_id.is_none() {
        invocation.explicit_name = Some(name.to_owned());
    }
    let client = Client::connect(&context.agentx_dir, invocation)?;
    match client.call(ServiceRequest::Participants(ParticipantRequest::Resolve))? {
        ServiceResponse::Participants(ParticipantResponse::Actor(actor)) => Ok((client, actor)),
        _ => bail!("unexpected process participant response"),
    }
}

impl ProcessHost {
    pub(crate) fn client(&self) -> &Client {
        &self.client
    }

    pub(crate) fn registration(&self) -> &ProcessRegistration {
        &self.registration
    }

    /// A Provider session switch can select a different participant. Preserve
    /// this host peer and child, and obtain a fresh participant fence explicitly.
    pub(crate) fn rebind(&mut self, participant: crate::agents::attachments::ParticipantRef) -> Result<()> {
        if self.registration.participant == participant {
            return Ok(());
        }
        let response = HostService::call(&self.client, Request::RegisterProcess { participant, child: self.registration.child.clone(), launch_context: None })?;
        match response {
            Response::ProcessRegistered { registration, display_name } => {
                self.registration = registration;
                self.display_name = display_name;
                Ok(())
            }
            _ => bail!("unexpected process rebind response"),
        }
    }
    /// Called while the host retains an unreaped child handle. Failure remains
    /// fatal to a tracked launch so it cannot leave an untracked child alive.
    pub(crate) fn register(context: &AgentxContext, name: &str, child_pid: u32, launch_context: Option<String>) -> Result<Self> {
        let (client, _) = connect(context, name)?;
        Self::register_with_client(&client, context, name, child_pid, launch_context)
    }

    /// Preserve an admitted operation's generation while resolving its host.
    pub(crate) fn register_with_client(client: &Client, context: &AgentxContext, name: &str, child_pid: u32, launch_context: Option<String>) -> Result<Self> {
        let mut invocation = super::host_client::invocation(context);
        if invocation.process_binding_id.is_none() {
            invocation.explicit_name = Some(name.to_owned());
        }
        if client.scope_root() != context.agentx_dir.canonicalize()?.as_path() {
            bail!("process client scope differs from context");
        }
        let client = client.with_invocation(invocation);
        let actor = match client.call(ServiceRequest::Participants(ParticipantRequest::Resolve))? {
            ServiceResponse::Participants(ParticipantResponse::Actor(actor)) => actor,
            _ => bail!("unexpected process participant response"),
        };
        let child = ChildProcess { pid: child_pid, incarnation: crate::sys::process::identity(child_pid) };
        match HostService::call(&client, Request::RegisterProcess { participant: actor.participant, child, launch_context })? {
            Response::ProcessRegistered { registration, display_name } => Ok(Self { client, registration, display_name, endpoints: Default::default() }),
            _ => bail!("unexpected process registration response"),
        }
    }

    #[cfg(windows)]
    pub(crate) fn capture_context(&self, launch_context: String) -> Result<()> {
        HostService::call(&self.client, Request::CaptureProcessContext { registration: self.registration.clone(), launch_context })?;
        Ok(())
    }

    pub(crate) fn set_endpoint(&self, kind: EndpointKind, port: u16) -> Result<()> {
        HostService::call(&self.client, Request::SetProcessEndpoint { registration: self.registration.clone(), kind: kind.clone(), port })?;
        if let Ok(mut endpoints) = self.endpoints.lock() {
            endpoints.retain(|(current, _)| current != &kind);
            endpoints.push((kind, port));
        }
        Ok(())
    }

    pub(crate) fn release_endpoint(&self, kind: EndpointKind, port: u16) -> Result<()> {
        HostService::call(&self.client, Request::ReleaseProcessEndpoint { registration: self.registration.clone(), kind, port })?;
        Ok(())
    }

    pub(crate) fn release_endpoints(&self) {
        let endpoints = self.endpoints.lock().map(|mut endpoints| std::mem::take(&mut *endpoints)).unwrap_or_default();
        for (kind, port) in endpoints {
            let _ = self.release_endpoint(kind, port);
        }
    }
}

/// Only an explicit approval edge is published. A hook's later status is
/// preserved atomically at the daemon; the returned current status drives UI.
pub(crate) fn observe_approval(context: &AgentxContext, name: &str, approval: bool) -> Result<String> {
    let (client, actor) = connect(context, name)?;
    match HostService::call(&client, Request::ObserveApproval { participant: actor.participant, approval })? {
        Response::ProcessStatus(status) => Ok(status),
        _ => bail!("unexpected approval observation response"),
    }
}
