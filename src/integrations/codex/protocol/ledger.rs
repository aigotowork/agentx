//! Bounded, independent ledgers for each direction of one native connection.

use std::collections::{HashMap, HashSet};

use super::{ClientRequestLedger, ClientRequestOwner, RouteFailure, RpcId, ServerRequestLedger};

/// A downstream ID is never reused, even after resolution, so a late response
/// cannot resolve a newer request. Capacity bounds only outstanding requests.
pub(crate) struct PendingClientRequests {
    pending: HashMap<RpcId, ClientRequestOwner>,
    outer_ids: HashSet<RpcId>,
    next: Option<i64>,
    capacity: usize,
    closed: bool,
}

impl PendingClientRequests {
    pub(crate) fn new(capacity: usize) -> Self {
        Self { pending: HashMap::new(), outer_ids: HashSet::new(), next: Some(1), capacity, closed: false }
    }

    pub(crate) fn close(&mut self) -> Vec<(RpcId, ClientRequestOwner)> {
        self.closed = true;
        self.outer_ids.clear();
        self.pending.drain().collect()
    }
}

impl ClientRequestLedger for PendingClientRequests {
    fn reserve(&mut self, owner: ClientRequestOwner) -> Result<RpcId, RouteFailure> {
        if self.closed {
            return Err(RouteFailure::Closed);
        }
        if let ClientRequestOwner::Outer { original_id } = &owner
            && self.outer_ids.contains(original_id)
        {
            return Err(RouteFailure::DuplicateOutstandingId);
        }
        if self.pending.len() >= self.capacity {
            return Err(RouteFailure::Capacity);
        }
        let sequence = self.next.ok_or(RouteFailure::Capacity)?;
        self.next = sequence.checked_add(1);
        let id = RpcId::Integer(sequence);
        if let ClientRequestOwner::Outer { original_id } = &owner {
            self.outer_ids.insert(original_id.clone());
        }
        self.pending.insert(id.clone(), owner);
        Ok(id)
    }

    fn resolve(&mut self, downstream_id: &RpcId) -> Result<ClientRequestOwner, RouteFailure> {
        if self.closed {
            return Err(RouteFailure::Closed);
        }
        let owner = self.pending.remove(downstream_id).ok_or(RouteFailure::UnknownResponseId)?;
        if let ClientRequestOwner::Outer { original_id } = &owner {
            self.outer_ids.remove(original_id);
        }
        Ok(owner)
    }
}

pub(crate) struct PendingServerRequests {
    pending: HashSet<RpcId>,
    capacity: usize,
    closed: bool,
}

impl PendingServerRequests {
    pub(crate) fn new(capacity: usize) -> Self {
        Self { pending: HashSet::new(), capacity, closed: false }
    }

    pub(crate) fn close(&mut self) {
        self.closed = true;
        self.pending.clear();
    }
}

impl ServerRequestLedger for PendingServerRequests {
    fn track(&mut self, native_id: RpcId) -> Result<(), RouteFailure> {
        if self.closed {
            return Err(RouteFailure::Closed);
        }
        if self.pending.contains(&native_id) {
            return Err(RouteFailure::DuplicateOutstandingId);
        }
        if self.pending.len() >= self.capacity {
            return Err(RouteFailure::Capacity);
        }
        self.pending.insert(native_id);
        Ok(())
    }

    fn resolve(&mut self, native_id: &RpcId) -> Result<(), RouteFailure> {
        if self.closed {
            return Err(RouteFailure::Closed);
        }
        if !self.pending.remove(native_id) {
            return Err(RouteFailure::UnknownResponseId);
        }
        Ok(())
    }
}
