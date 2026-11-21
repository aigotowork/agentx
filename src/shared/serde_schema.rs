//! Schema counterparts for standard serde representations unsupported by utoipa.
//! These types are schema-only; native values and serializers remain authoritative.

/// serde serializes Result<T, E> as the externally tagged Ok/Err union.
/// Use only as a ToSchema value_type override; never convert runtime results to it.
#[allow(dead_code)]
#[derive(utoipa::ToSchema)]
pub(crate) enum SerdeResult<T, E> {
    Ok(T),
    Err(E),
}
