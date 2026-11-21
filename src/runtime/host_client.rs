//! Local IPC implementation of runtime-host capabilities. The client captures
//! configuration once and never opens AgentX storage after transport failure.

use crate::service::{InvocationContext, Request, Response, ServiceError, ServiceResult};

use super::context::AgentxContext;
use super::hosts::{HostService, Request as HostRequest, Response as HostResponse};

/// Preserve deletions and overrides by sending the captured caller map; do not
/// reconstruct a session from the daemon's environment.
pub(crate) fn invocation(context: &AgentxContext) -> InvocationContext {
    InvocationContext {
        cwd: context.cwd.clone(),
        environment: context.raw_env.clone(),
        environment_file_keys: context.environment_file_keys.clone(),
        process_binding_id: context.process_id.clone(),
        stdin_is_tty: context.stdin_is_tty,
        stdout_is_tty: context.stdout_is_tty,
        ..Default::default()
    }
}

impl HostService for crate::service::client::Client {
    fn call(&self, request: HostRequest) -> ServiceResult<HostResponse> {
        match crate::service::client::Client::call(self, Request::Hosts(request))? {
            Response::Hosts(response) => Ok(response),
            _ => Err(ServiceError::Internal("daemon returned a different runtime capability".into())),
        }
    }
}
