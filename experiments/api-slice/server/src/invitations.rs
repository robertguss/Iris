use crate::{Actor, ApiError, AppState, ErrorCode, Problem, database_error};
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use iris_sqlite_spike::{IssueInvitation, IssueOutcome, connect, issue};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, utoipa::ToSchema, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IssueRequest {
    /// Positive signed-64-bit decimal ID. No leading zeros.
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    #[schemars(regex(pattern = "^[1-9][0-9]*$"), length(max = 19))]
    pub project_id: String,
    /// Existing account; same ID format as project_id.
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    #[schemars(regex(pattern = "^[1-9][0-9]*$"), length(max = 19))]
    pub recipient_id: String,
}

#[derive(Serialize, utoipa::ToSchema, schemars::JsonSchema)]
pub struct IssuedInvitation {
    pub project_id: String,
    pub recipient_id: String,
    /// Demo-only delivery: treat as a credential. Stored only as a hash.
    pub token: String,
    /// Unix seconds, serialized as a string.
    pub expires_at: String,
}

fn id(value: &str) -> Result<i64, ApiError> {
    let parsed = value
        .parse::<i64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == value);
    parsed.ok_or(ApiError(ErrorCode::InvalidRequest))
}

#[utoipa::path(
    post, path = "/api/invitations", operation_id = "issueInvitation",
    request_body = IssueRequest,
    responses(
        (status = 201, description = "Invitation issued; membership unchanged", body = IssuedInvitation),
        (status = 400, description = "Invalid request", body = Problem),
        (status = 401, description = "Sign-in required", body = Problem),
        (status = 403, description = "Not a project owner or CSRF validation failed", body = Problem),
        (status = 404, description = "Recipient not found", body = Problem),
        (status = 409, description = "Already a member or invitation pending", body = Problem),
        (status = 500, description = "Internal error", body = Problem),
        (status = 503, description = "Database busy", body = Problem)
    ), security(("BrowserSession" = []))
)]
pub async fn issue_endpoint(
    State(state): State<AppState>,
    Actor(actor_id): Actor,
    body: Result<Json<IssueRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(body) = body.map_err(|_| ApiError(ErrorCode::InvalidRequest))?;
    let input = IssueInvitation {
        project_id: id(&body.project_id)?,
        recipient_id: id(&body.recipient_id)?,
        actor_id,
        now: (state.now)(),
    };
    let mut conn = connect(&state.database).await.map_err(database_error)?;
    match issue(&mut conn, input).await.map_err(database_error)? {
        IssueOutcome::Issued { token, expires_at } => Ok((
            StatusCode::CREATED,
            [(header::CACHE_CONTROL, "no-store")],
            Json(IssuedInvitation {
                project_id: body.project_id,
                recipient_id: body.recipient_id,
                token,
                expires_at: expires_at.to_string(),
            }),
        )
            .into_response()),
        IssueOutcome::Forbidden => Err(ApiError(ErrorCode::Forbidden)),
        IssueOutcome::RecipientNotFound => Err(ApiError(ErrorCode::RecipientNotFound)),
        IssueOutcome::AlreadyMember => Err(ApiError(ErrorCode::AlreadyMember)),
        IssueOutcome::InvitationPending => Err(ApiError(ErrorCode::InvitationPending)),
    }
}
