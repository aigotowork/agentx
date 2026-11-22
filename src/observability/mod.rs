//! Bounded diagnostic evidence, separate from business and transport authority.
//!
//! See `AGENTS.md` for ownership and `.docs/observability/README.md` for the
//! selected contracts, scenarios and implementation acceptance boundary.

pub(crate) mod configuration;
mod configuration_store;
pub(crate) mod context;
pub(crate) mod events;
mod journal;
pub(crate) mod post_commit;
mod registry;
pub(crate) mod runtime;
pub(crate) mod trace;
pub(crate) mod web;
