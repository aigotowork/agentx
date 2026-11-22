//! Consumer management capability, independent of HTTP parsing and serialization.
//! Captured host context supplies the scope/project/environment; the daemon remains
//! the sole identity, journal and policy owner. Runtime effects run in a child host.
use super::contract::*;
use crate::service::{InvocationContext, ServiceError, ServiceResult, client::Client};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::Semaphore;

mod inspection;
mod resources;
mod supervision;

/// Internal disk-read product; only metadata and bytes cross the HTTP adapter.
/// Stored host paths never cross this boundary.
pub(crate) struct AttachmentBytes {
    pub attachment: Attachment,
    pub bytes: Vec<u8>,
}

/// Native lifecycle intent; both HTTP kill/stop aliases map to Kill.
/// This enum is host-owned policy selection, not a remote generic-command endpoint.
pub(crate) enum TargetAction {
    Kill,
    StopParticipation,
    Fork,
    Resume,
    Transcript,
}

/// Reuse one worker semaphore across listener reconfiguration and native callers.
/// Construction accepts trusted host configuration, never remote caller claims.
pub(crate) struct ManagementService {
    scope: PathBuf,
    project: PathBuf,
    invocation: InvocationContext,
    workers: Arc<Semaphore>,
}
impl ManagementService {
    pub(crate) fn new(scope: PathBuf, project: PathBuf, invocation: InvocationContext, workers: Arc<Semaphore>) -> Self {
        Self { scope, project, invocation, workers }
    }

    /// Bounded native Agent page; this read does not run maintenance.
    pub(crate) fn list_agents(&self, input: ListAgentsInput) -> ServiceResult<AgentPage> {
        let request = list_request(&input)?;
        match self.call(&input.generation, request)? {
            Response::Agents(page) => Ok(page),
            _ => Err(ServiceError::Internal("unexpected Agent page response".into())),
        }
    }

    /// Bounded native membership page over matching adopted snapshots only.
    pub(crate) fn list_workspace_members(&self, input: ListWorkspaceMembersInput) -> ServiceResult<AgentPage> {
        let limit = resources::page_limit(input.limit)?;
        let request = Request::ListWorkspaceMembers { workspace: input.workspace, current_only: input.current_only, after: input.after, limit };
        match self.call(&input.generation, request)? {
            Response::Agents(page) => Ok(page),
            _ => Err(ServiceError::Internal("unexpected workspace member page response".into())),
        }
    }

    /// Exact incarnation or stopped event; never fall back to a same-name successor.
    pub(crate) fn get_agent(&self, input: GetAgentInput) -> ServiceResult<AgentView> {
        match self.call(&input.generation, Request::Agent(input.agent))? {
            Response::Agent(agent) => Ok(agent),
            _ => Err(ServiceError::Internal("unexpected Agent response".into())),
        }
    }

    /// Acceptance returns the durable receipt, not evidence of delivery or a reply.
    /// Reuse the original ID, generation and complete payload after transport loss.
    pub(crate) fn submit_message(&self, input: SubmitMessage) -> ServiceResult<Operation> {
        self.submit_command(&input.generation, input.request_id, Command::Send { agent: input.agent, message: input.message })
    }

    /// Observe/reconcile the original receipt; absence is not proof of no effect.
    pub(crate) fn get_operation(&self, input: GetOperationInput) -> ServiceResult<Operation> {
        match self.call(&input.generation, Request::Operation { request_id: input.request_id })? {
            Response::Operation(operation) => Ok(operation),
            _ => Err(ServiceError::Internal("unexpected operation response".into())),
        }
    }

    pub(crate) fn submit_launch(&self, input: SubmitLaunch) -> ServiceResult<Operation> {
        self.submit_command(&input.generation, input.request_id, Command::Launch(input.launch))
    }

    pub(crate) fn submit_directory_message(&self, input: SubmitDirectoryMessage) -> ServiceResult<Operation> {
        self.submit_command(&input.generation, input.request_id, Command::DirectorySend { directory: input.directory, message: input.message })
    }

    pub(crate) fn patch_labels(&self, input: SubmitLabels) -> ServiceResult<Operation> {
        self.submit_command(&input.generation, input.request_id, Command::PatchLabels(LabelsPatch { agent: input.agent, set: input.set, remove: input.remove, if_revision: input.if_revision }))
    }

    pub(crate) fn submit_target(&self, input: SubmitTarget, action: TargetAction) -> ServiceResult<Operation> {
        let command = match action {
            TargetAction::Kill => Command::Stop(input.agent),
            TargetAction::StopParticipation => Command::StopParticipation(input.agent),
            TargetAction::Fork => Command::Fork(input.agent),
            TargetAction::Resume => Command::Resume(input.agent),
            TargetAction::Transcript => Command::Transcript(input.agent),
        };
        self.submit_command(&input.generation, input.request_id, command)
    }

    pub(crate) fn client(&self, generation: Option<&str>) -> Result<Client, ServiceError> {
        let client = Client::connect_existing(&self.scope, self.invocation.clone())?;
        if generation.is_some_and(|expected| expected != client.info().database_generation) {
            return Err(ServiceError::StaleFence);
        }
        Ok(client)
    }
    pub(crate) fn call(&self, generation: &str, request: Request) -> Result<Response, ServiceError> {
        let client = self.client(Some(generation))?;
        match client.call(crate::service::Request::Management(request))? {
            crate::service::Response::Management(result) => Ok(result),
            _ => Err(ServiceError::Internal("unexpected management response".into())),
        }
    }
    pub(crate) fn describe(&self) -> Result<Descriptor, ServiceError> {
        let client = self.client(None)?;
        let catalog = crate::config::profiles::ProfileCatalog::load(&self.scope).map_err(|e| ServiceError::InvalidInput(e.to_string()))?;
        Ok(Descriptor {
            supports_labels: true,
            supports_workspace_members: client.info().workspace_members,
            supports_message_history: client.info().message_history,
            generation: client.info().database_generation.clone(),
            scope: self.scope.to_string_lossy().into(),
            project: self.project.to_string_lossy().into(),
            tools: crate::integrations::registry::ALL.iter().filter(|spec| spec.released).map(|spec| spec.name.into()).collect(),
            profiles: catalog.profiles.into_iter().map(|(name, profile)| ProfileView { name, tool: profile.tool, description: profile.description.unwrap_or_default() }).collect(),
            max_page: MAX_PAGE,
            max_text: MAX_TEXT as u32,
        })
    }
    pub(crate) fn submit_command(&self, generation: &str, request_id: String, command: Command) -> Result<Operation, ServiceError> {
        // Retain the public command exactly. Resolving a changed/removed path
        // during a replay must not prevent retrieval of its original receipt.
        let mut working_directory = self.project.clone();
        if let Command::DirectorySend { directory, .. } = &command {
            working_directory = PathBuf::from(directory);
        }
        if let Command::Launch(input) = &command {
            let requested = input.directory.as_ref().map(PathBuf::from).unwrap_or_else(|| self.project.clone());
            working_directory = if requested.is_absolute() { requested } else { self.project.join(requested) };
        }
        let permit = self.workers.clone().try_acquire_owned().ok();
        let client = self.client(Some(generation))?;
        let crate::service::Response::Management(Response::Grant(grant)) = client.call(crate::service::Request::Management(Request::Submit { request_id: request_id.clone(), command, admit: permit.is_some() }))? else {
            return Err(ServiceError::Internal("unexpected admission response".into()));
        };
        if let Some(ticket) = grant.ticket {
            let mut invocation = self.invocation.clone();
            invocation.cwd = working_directory;
            let input = WorkerInput { scope: self.scope.to_string_lossy().into(), generation: generation.into(), request_id, ticket, invocation };
            std::thread::spawn(move || {
                let _permit = permit;
                let failure = supervision::run_worker(&input).err();
                if let Some(reason) = failure {
                    crate::log::log_in_scope(client.scope_root(), "ERROR", "management", "worker.failed", &reason, &[("generation", &input.generation), ("operation_id", &input.request_id)]);
                    if let Err(error) = client.call(crate::service::Request::Management(Request::Abandon { request_id: input.request_id.clone(), ticket: input.ticket, reason })) {
                        crate::log::log_in_scope(client.scope_root(), "ERROR", "management", "worker.abandon_failed", &error.to_string(), &[("generation", &input.generation), ("operation_id", &input.request_id)]);
                    }
                }
            });
        }
        Ok(grant.operation)
    }
}
/// Only argument normalization lives here; selector/cursor/reference policy stays native.
fn list_request(input: &ListAgentsInput) -> ServiceResult<Request> {
    let limit = resources::page_limit(input.limit)?;
    if !input.current_only && !input.labels.is_empty() {
        return Err(ServiceError::InvalidInput("label selectors require current_only=true".into()));
    }
    Ok(if input.current_only { Request::SelectLabels { after: input.after.clone(), limit, selectors: input.labels.clone() } } else { Request::List { after: input.after.clone(), limit } })
}

#[cfg(test)]
mod tests;
