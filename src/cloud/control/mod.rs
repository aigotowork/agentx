//! Typed cloud control. Both HTTP projections use the same persisted Namespace authority.
mod http;
mod store;
pub(super) mod types;
mod workspaces;

pub(super) use http::{admin_document, admin_router, namespace_document, namespace_router};
pub(super) use workspaces::observe_workspace_availability;

pub(super) const SCHEMA: &str = include_str!("schema.sql");
