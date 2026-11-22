//! Message routing, validation, bundles and delivery.
pub(crate) mod activation;
pub(crate) mod bundle_service;
mod bundle_store;
pub mod bundles;
pub(crate) mod delivery;
pub(crate) mod delivery_report;
pub(crate) mod delivery_service;
pub(crate) mod delivery_store;
pub(crate) mod formatting;
pub(crate) mod inbox;
mod inbox_store;
pub mod routing;
pub(crate) mod service;
mod service_remote;
mod service_store;
pub(crate) mod submission;
pub(crate) mod tips;
pub mod validation;
