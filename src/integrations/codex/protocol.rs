//! Request ownership for one native client connection. Native payload DTOs stay
//! owned by Codex; the bridge preserves unknown payload fields as raw JSON.

use crate::agents::attachments::ActivationTarget;
use crate::messaging::activation::AttemptId;

mod envelope;
mod ledger;
mod router;
mod subscriptions;

#[cfg(test)]
mod acceptance_tests;

pub(crate) use envelope::{FrameKind, NativeFrame, ProtocolFailure, RawObject};
pub(crate) use ledger::{PendingClientRequests, PendingServerRequests};
pub(crate) use router::{NativeRoute, OuterRoute, ProtocolRouter};
pub(crate) use subscriptions::ConnectionHandshake;

/// Native RequestId is string or int64; `1` and `"1"` are distinct identities.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RpcId {
    Integer(i64),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ClientRequestOwner {
    Outer { original_id: RpcId },
    Activation { target: ActivationTarget, attempt: AttemptId },
    Reconciliation { target: ActivationTarget },
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RouteFailure {
    #[error("duplicate outstanding request id")]
    DuplicateOutstandingId,
    #[error("unknown response id")]
    UnknownResponseId,
    #[error("native request ledger capacity reached")]
    Capacity,
    #[error("native request ledger closed")]
    Closed,
}

/// All client requests share one downstream allocator, including unknown outer
/// methods. Rewriting only an internal prefix cannot prevent caller collisions.
/// Resolving removes exactly one outstanding owner; internal responses never
/// become outer responses. Root-intent correlation is bridge-owned metadata.
pub(crate) trait ClientRequestLedger {
    fn reserve(&mut self, owner: ClientRequestOwner) -> Result<RpcId, RouteFailure>;
    fn resolve(&mut self, downstream_id: &RpcId) -> Result<ClientRequestOwner, RouteFailure>;
}

/// Independent server-to-client direction, retaining original native ids and
/// embedded `serverRequest/resolved.requestId`. AgentX does not invent requests
/// to the outer client. Track all native requests, including unknown methods.
pub(crate) trait ServerRequestLedger {
    fn track(&mut self, native_id: RpcId) -> Result<(), RouteFailure>;
    fn resolve(&mut self, native_id: &RpcId) -> Result<(), RouteFailure>;
}
