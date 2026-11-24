//! Native AgentX application boundary, independent of CLI syntax and local IPC.
//! Domain-owned request/outcome types are serialized directly by the transport.
//! See `AGENTS.md` and `.docs/architecture/contracts.md` before extending it.

pub(crate) mod application;
pub(crate) mod client;
pub(crate) mod codex_hook;
pub(crate) mod context;
pub(crate) mod error;
pub(crate) mod maintenance;
pub(crate) mod maintenance_store;
pub(crate) mod observation;
pub(crate) mod queries;
pub(crate) mod queries_store;
mod read_application;

pub(crate) use context::{CallerContext, CallerProcess, InvocationContext, RequestContext, RequestId};
pub(crate) use error::{ServiceError, ServiceResult};

/// Selected local service operations. Each variant belongs to its domain owner.
/// Adding a wire operation extends this native declaration, never a second IDL.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[allow(clippy::large_enum_variant, reason = "Keep the native domain payload inline in this serialized envelope")]
pub(crate) enum Request {
    Inspection(crate::inspection::contract::Request),
    Management(crate::management::contract::Request),
    Collaboration(crate::collaboration::service::Request),
    Participants(crate::agents::service::Request),
    CodexHook(codex_hook::HookEvent),
    Messages(crate::messaging::service::Request),
    Hosts(crate::runtime::hosts::Request),
    Events(crate::events::service::Request),
    Queries(queries::Request),
    Maintenance(maintenance::Request),
    Relay(crate::relay::service::Request),
}

/// The domain outcome, distinct from transport acceptance and external effects.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[expect(clippy::large_enum_variant, reason = "One inline domain outcome per bounded IPC reply; preserve the finite native envelope without lint-only boxing")]
pub(crate) enum Response {
    Inspection(crate::inspection::contract::Response),
    Management(crate::management::contract::Response),
    Collaboration(crate::collaboration::service::Response),
    Participants(crate::agents::service::Response),
    CodexHook(codex_hook::HookOutcome),
    Messages(crate::messaging::service::Response),
    Hosts(crate::runtime::hosts::Response),
    Events(crate::events::service::Response),
    Queries(queries::Response),
    Maintenance(maintenance::Response),
    Relay(crate::relay::service::Response),
}

/// Daemon composition injects its application into the local request server.
/// Implementations may own a non-`Sync` SQLite connection on one dispatch worker.
/// Provider process I/O must not run inside a write transaction.
pub(crate) trait Application {
    fn handle(&mut self, context: &RequestContext, request: Request) -> ServiceResult<Response>;

    /// Run at most a bounded amount of derived maintenance when no request is
    /// waiting. Authoritative request handling must never depend on completion.
    fn idle(&mut self) {}

    /// Bounded work that must progress even while the writer has queued calls.
    fn after_request(&mut self) {}

    fn read_lane(&self) -> Option<ReadLane> {
        None
    }
}

pub(crate) struct ReadLane {
    pub accepts: fn(&Request) -> bool,
    pub application: Box<dyn Application + Send>,
}
