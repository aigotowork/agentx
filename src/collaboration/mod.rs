//! Bounded assignment-to-result/check/acceptance capability. See AGENTS.md.
mod model;
pub(crate) mod service;
mod store;
pub(crate) mod web;

#[cfg(test)]
mod tests;
