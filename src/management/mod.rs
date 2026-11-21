//! Agent management application composition; see AGENTS.md and the native contract.
mod activity;
pub(crate) mod contract;
mod human;
pub(crate) mod offices;
pub(crate) mod openapi;
pub(crate) mod service;
pub(crate) mod store;
pub(crate) mod web;
pub(crate) mod worker;

mod uploads;

pub(crate) mod relations;
