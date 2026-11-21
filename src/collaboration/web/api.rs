//! Closed browser operations: operator actions only; no Agent report authority.
use crate::collaboration::service::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct Limits {
    pub text: u32,
    pub title: u32,
    pub source_reference: u32,
    pub source_revision: u32,
    pub criteria: u32,
    pub artifacts: u32,
    pub page: u32,
    pub history: u32,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
pub(crate) struct Descriptor {
    pub scope: String,
    pub project: String,
    pub project_label: String,
    pub generation: Option<String>,
    pub collaboration: bool,
    pub available: bool,
    pub limits: Limits,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadCall {
    pub generation: String,
    pub read: Read,
}
#[derive(Serialize, Deserialize, Clone, Debug, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplyCall {
    pub generation: String,
    pub command_id: String,
    pub work: Option<WorkRef>,
    pub expected_revision: Option<u64>,
    pub command: OperatorCommand,
}
