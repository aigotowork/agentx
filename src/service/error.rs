//! Actionable application failures. Transport loss remains a separate concern.

pub(crate) type ServiceResult<T> = Result<T, ServiceError>;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ServiceError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("identity required: {0}")]
    IdentityRequired(String),
    #[error("caller is not authorized: {0}")]
    Forbidden(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("stale participant, execution, attachment or database generation")]
    StaleFence,
    #[error("database generation changed; explicitly reconnect before issuing new work")]
    StaleGeneration,
    #[error("service unavailable: {0}")]
    Unavailable(String),
    #[error("unsupported operation or capability: {0}")]
    Unsupported(String),
    #[error("operation outcome is unknown; reconcile before retry: {0}")]
    OutcomeUnknown(String),
    #[error("internal application failure: {0}")]
    Internal(String),
}
