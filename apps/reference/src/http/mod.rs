//! Application HTTP adapters. Shared envelope, bridge, boundary and assembly
//! checks live in the `iris` crate; session markers and wire IDs stay here.
pub mod memberships;
pub mod projects;

use crate::lifecycle::Tracked;
use crate::{
    app::AppState,
    domains::FailureKind,
    identity::{Auth, ErrorCode},
    read::{ReadError, Stop},
};
use axum::{Router, response::Response};
use iris::{Mapping, Operation, RequestId, Shared, shared};
use serde::{Deserialize, Serialize};

/// Positive canonical decimal IDs only; the caller maps failure to its refusal.
pub(crate) fn parse_id(value: &str) -> Result<i64, ()> {
    let parsed = value
        .parse::<i64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == value);
    parsed.ok_or(())
}

/// A fresh connection, tracked until its closure is acknowledged, or the
/// shared failure for not opening one.
pub(crate) async fn open(state: &AppState) -> Result<Tracked, Shared> {
    state.connections.open(&state.database).await.map_err(|e| {
        if crate::app::is_busy(&e) {
            Shared::Unavailable
        } else {
            Shared::Internal
        }
    })
}

/// Page sizes are canonical decimals from 1 to 100, so `05` is refused;
/// absent means 50. The exported pattern states the same rule.
pub(crate) fn parse_limit(value: Option<&str>) -> Result<u32, ()> {
    let Some(value) = value else { return Ok(50) };
    let parsed = value
        .parse::<u32>()
        .ok()
        .filter(|n| (1..=100).contains(n) && n.to_string() == value);
    parsed.ok_or(())
}

/// Cursors are opaque to clients, versioned and at most 32 bytes; today's
/// canonical key already keeps them within 22, so the explicit bound is the
/// documented one rather than an observable check. They carry a position only:
/// authority comes from the actor, so a forged or foreign cursor can
/// reposition within visible rows but never widen them.
const CURSOR_VERSION: &str = "c1.";

pub(crate) fn encode_cursor(after: i64) -> String {
    format!("{CURSOR_VERSION}{after}")
}

pub(crate) fn decode_cursor(value: &str) -> Result<i64, ()> {
    if value.len() > 32 {
        return Err(());
    }
    value
        .strip_prefix(CURSOR_VERSION)
        .ok_or(())
        .and_then(parse_id)
}

/// Unknown and duplicate parameters are refused, as bodies refuse unknown
/// fields.
#[derive(Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(crate) struct PageQuery {
    /// Page size from 1 to 100, written without leading zeros; defaults to 50.
    #[param(pattern = "^(100|[1-9][0-9]?)$")]
    pub limit: Option<String>,
    /// An earlier page's `next_cursor`, echoed verbatim.
    #[param(max_length = 32, pattern = "^c1\\.[1-9][0-9]*$")]
    pub cursor: Option<String>,
}

pub(crate) const PAGE_READ: Mapping = Mapping {
    status: 200,
    kind: "success",
    code: None,
    message: "Page read",
    rule: None,
    prerequisite: None,
};

/// A read's success, rejection or failure; busy is the only 503.
pub(crate) fn read_reply<R: Copy, T: Serialize>(
    op: &Operation<R>,
    id: &RequestId,
    result: Result<T, ReadError<R>>,
) -> Response {
    match result {
        Ok(page) => op.render(&op.success, id, Some(serde_json::to_value(page).unwrap())),
        Err(ReadError::Rejected(r)) => op.render(&(op.rejection)(r), id, None),
        Err(ReadError::Failed {
            primary:
                Stop::Execution {
                    kind: FailureKind::Busy,
                    ..
                },
            ..
        }) => op.render(&shared(Shared::Unavailable), id, None),
        Err(ReadError::Failed { .. }) => op.render(&shared(Shared::Internal), id, None),
    }
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
