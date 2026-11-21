//! Typed client binding used by MQTT workers and local relay observers.
use super::service;
use crate::service::{InvocationContext, Request, Response, ServiceError, ServiceResult, client::Client};

pub(crate) fn connect() -> ServiceResult<Client> {
    Client::connect(crate::paths::agentx_dir(), InvocationContext::capture()?)
}
pub(crate) fn call(client: &Client, request: service::Request) -> ServiceResult<service::Response> {
    match client.call(Request::Relay(request))? {
        Response::Relay(response) => Ok(response),
        _ => Err(ServiceError::Internal("unexpected relay response".into())),
    }
}
pub(crate) fn request(request: service::Request) -> ServiceResult<service::Response> {
    call(&connect()?, request)
}
pub(crate) fn worker_status(status: &str, error: Option<&str>) {
    if let Err(error) = request(service::Request::WorkerStatus { status: status.into(), error: error.map(str::to_owned) }) {
        crate::log::log_warn("relay", "relay.status_write_failed", &error.to_string());
    }
}
