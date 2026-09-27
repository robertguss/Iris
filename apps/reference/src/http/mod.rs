//! Application HTTP adapters. Shared envelope, bridge, boundary and assembly
//! checks live in the `iris` crate; session markers and wire IDs stay here.
pub mod memberships;

use crate::{
    app::AppState,
    identity::{Auth, ErrorCode},
};
use axum::{Router, response::Response};
use iris::Shared;

/// Positive canonical decimal IDs only; the caller maps failure to its refusal.
pub(crate) fn parse_id(value: &str) -> Result<i64, ()> {
    let parsed = value
        .parse::<i64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == value);
    parsed.ok_or(())
}

/// Maps the session layer's private refusal markers to the shared profile.
pub(crate) fn classify(response: &Response) -> Option<Shared> {
    match response.extensions().get::<ErrorCode>() {
        Some(ErrorCode::Csrf) => Some(Shared::Csrf),
        Some(ErrorCode::Unauthorized) => Some(Shared::Unauthenticated),
        Some(ErrorCode::Internal) => Some(Shared::Internal),
        _ => None,
    }
}

/// Applies the session stack, then this operation's boundary, to its router.
pub(crate) type Mount = fn(Router<AppState>, Auth) -> Router<AppState>;
