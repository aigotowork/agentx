//! Typed cloud control. Both HTTP projections use the same persisted scope authority.
mod http;
mod store;
pub(super) mod types;

pub(super) use http::{admin_document, admin_router, scope_document, scope_router};
