//! CLI commands for agentx.
//!

// Messaging
pub mod listen;
pub(crate) mod listen_client;
pub mod send;

// Lifecycle
pub(crate) mod attach;
pub mod daemon;
pub mod fork;
pub mod kill;
pub mod launch;
pub(crate) mod profile;
pub(crate) mod recover;
pub mod resume;
pub(crate) mod roles;
pub mod start;
pub mod stop;
mod target_selection;

// Diagnostics
pub mod bundle;
pub mod events;
pub(crate) mod events_client;
pub mod list;
pub(crate) mod observe;
pub mod status;
pub mod term;
pub mod transcript;

// Management
pub mod archive;
pub mod config;
pub(crate) mod docs;
pub mod help;
pub mod hooks;
pub mod relay;
pub mod reset;

pub mod run;
pub mod update;
pub(crate) mod work;

pub(crate) mod bindings;

pub(crate) mod labels;
