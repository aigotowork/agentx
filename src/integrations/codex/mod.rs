//! Codex-specific shared service, root mapping and native protocol adaptation.
//! Contracts and write ownership: `.controller/codex-shared-runtime/contracts-handoff.md`.
//! Generic identity/mail policy must not import this module. The launcher selects
//! separate contracts for bridge clients and direct native terminals.

pub(crate) mod bridge;
pub(crate) mod configuration;
mod mail_evidence;
#[cfg(target_os = "linux")]
pub(crate) use mail_evidence::item_matches as mailbox_item_matches;
#[cfg(target_os = "linux")]
pub(crate) mod product_execution;
pub(crate) mod protocol;
pub(crate) mod roots;
pub(crate) mod service;
#[cfg(target_os = "linux")]
pub(crate) mod terminal;
