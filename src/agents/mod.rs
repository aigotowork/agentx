//! Participant records, identity, binding, lifecycle and naming.
pub(crate) mod attachments;
mod attachments_store;
pub(crate) mod binding;
pub(crate) mod claude_service;
mod claude_store;
mod cleanup_store;
pub(crate) mod copilot_service;
mod copilot_store;
pub(crate) mod hermes_service;
mod hermes_store;
pub(crate) mod hook_service;
mod hook_store;
pub mod identity;
pub(crate) mod invocation_binding;
pub(crate) mod labels;
mod labels_store;
pub(crate) mod launch_service;
mod launch_store;
pub(crate) mod launch_title;
pub(crate) mod lifecycle;
pub(crate) mod lifecycle_service;
mod lifecycle_store;
pub mod model_request;
mod model_request_store;
pub(crate) mod names;
pub(crate) mod records;
pub(crate) mod recovery;
pub(crate) mod recovery_store;
mod service_remote;
mod service_store;
pub(crate) mod workspaces;

pub(crate) mod service;

pub(crate) mod context_service;
pub(crate) mod context_store;

pub(crate) mod launch_relations;
