//! Agent launch, execution environment, terminal I/O and process notifications.
pub(crate) mod agent_context;
pub(crate) mod bootstrap;
pub(crate) mod codex_project_env;
pub(crate) mod codex_stdio;
pub(crate) mod config_effects;
pub mod context;
pub(crate) mod env;
pub(crate) mod environment;
pub(crate) mod host_client;
pub(crate) mod hosts;
pub(crate) mod hosts_store;
pub mod launch_status;
pub mod launcher;
pub(crate) mod notify;
pub(crate) mod poll_progress;
pub(crate) mod process_host;
mod process_host_store;
pub(crate) mod process_tracking;
pub(crate) mod pty;
pub(crate) mod shell_env;
pub mod terminal;
pub(crate) mod tmux;

pub(crate) mod terminal_client;

pub(crate) mod kill_client;
pub(crate) mod stop_effects;

pub(crate) mod launch_options;

pub(crate) mod launch_profile;
pub(crate) mod resume_client;

/// Shared native directory addressing and endpoint launch arbitration.
pub(crate) mod directory;
pub(crate) mod inspection;
