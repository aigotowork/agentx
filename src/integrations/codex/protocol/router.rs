//! Envelope routing; thread lifecycle and message admission remain bridge-owned.

use super::{ClientRequestLedger, ClientRequestOwner, FrameKind, NativeFrame, PendingClientRequests, PendingServerRequests, ProtocolFailure, RouteFailure, RpcId, ServerRequestLedger};

#[derive(Clone, Debug)]
pub(crate) enum OuterRoute {
    Request { downstream_id: RpcId, method: String, frame: String },
    Notification { method: String, frame: String },
    ServerResponse { native_id: RpcId, frame: String },
}

#[derive(Clone, Debug)]
pub(crate) enum NativeRoute {
    OuterResponse {
        downstream_id: RpcId,
        // Production consumes the restored frame; tests also observe ownership.
        #[cfg(test)]
        original_id: RpcId,
        frame: String,
    },
    InternalResponse {
        downstream_id: RpcId,
        owner: ClientRequestOwner,
        frame: String,
    },
    ServerRequest {
        native_id: RpcId,
        method: String,
        frame: String,
    },
    Notification {
        method: String,
        frame: String,
    },
}

/// One router per native connection. It never creates an RPC toward the outer
/// client and never mixes server request IDs into the downstream allocator.
pub(crate) struct ProtocolRouter {
    clients: PendingClientRequests,
    servers: PendingServerRequests,
}

impl ProtocolRouter {
    pub(crate) fn new(capacity: usize) -> Self {
        Self { clients: PendingClientRequests::new(capacity), servers: PendingServerRequests::new(capacity) }
    }

    pub(crate) fn route_outer(&mut self, raw: &str) -> Result<OuterRoute, ProtocolFailure> {
        let message = NativeFrame::parse(raw)?;
        match &message.kind {
            FrameKind::Request { id, method } => {
                let downstream_id = self.clients.reserve(ClientRequestOwner::Outer { original_id: id.clone() })?;
                Ok(OuterRoute::Request { frame: message.with_id(&downstream_id)?, downstream_id, method: method.clone() })
            }
            FrameKind::Notification { method } => Ok(OuterRoute::Notification { method: method.clone(), frame: message.raw().to_owned() }),
            FrameKind::Response { id } => {
                // Native may already have emitted serverRequest/resolved. A
                // late client response still belongs to the native protocol.
                if let Err(error) = self.servers.resolve(id)
                    && error != RouteFailure::UnknownResponseId
                {
                    return Err(error.into());
                }
                Ok(OuterRoute::ServerResponse { native_id: id.clone(), frame: message.raw().to_owned() })
            }
        }
    }

    pub(crate) fn reserve_internal(&mut self, owner: ClientRequestOwner) -> Result<RpcId, RouteFailure> {
        self.clients.reserve(owner)
    }

    /// Only for a request proven not to have reached the native writer.
    pub(crate) fn abandon_before_send(&mut self, id: &RpcId) -> Result<ClientRequestOwner, RouteFailure> {
        self.clients.resolve(id)
    }

    pub(crate) fn route_native(&mut self, raw: &str) -> Result<NativeRoute, ProtocolFailure> {
        let message = NativeFrame::parse(raw)?;
        match &message.kind {
            FrameKind::Response { id } => match self.clients.resolve(id)? {
                ClientRequestOwner::Outer { original_id } => Ok(NativeRoute::OuterResponse {
                    downstream_id: id.clone(),
                    frame: message.with_id(&original_id)?,
                    #[cfg(test)]
                    original_id,
                }),
                owner => Ok(NativeRoute::InternalResponse { downstream_id: id.clone(), owner, frame: message.raw().to_owned() }),
            },
            FrameKind::Request { id, method } => {
                self.servers.track(id.clone())?;
                Ok(NativeRoute::ServerRequest { native_id: id.clone(), method: method.clone(), frame: message.raw().to_owned() })
            }
            FrameKind::Notification { method } => {
                if method == "serverRequest/resolved"
                    && let Some(params) = message.params()?
                    && let Some(id) = params.get("requestId")
                {
                    let _ = self.servers.resolve(&RpcId::from_raw(id)?);
                }
                Ok(NativeRoute::Notification { method: method.clone(), frame: message.raw().to_owned() })
            }
        }
    }

    pub(crate) fn close(&mut self) -> Vec<(RpcId, ClientRequestOwner)> {
        self.servers.close();
        self.clients.close()
    }
}
