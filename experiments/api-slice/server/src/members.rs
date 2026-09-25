use crate::{Actor, ApiError, AppState, ErrorCode, Problem, database_error};
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
};
use iris_sqlite_spike::{ChangeMember, MemberOutcome, MemberRole, change_member, connect};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize, Serialize, utoipa::ToSchema, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Editor,
    Viewer,
}

#[derive(Deserialize, utoipa::ToSchema, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangeRoleRequest {
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    #[schemars(regex(pattern = "^[1-9][0-9]*$"), length(max = 19))]
    pub project_id: String,
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    #[schemars(regex(pattern = "^[1-9][0-9]*$"), length(max = 19))]
    pub user_id: String,
    pub role: Role,
}

#[derive(Deserialize, utoipa::ToSchema, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveMemberRequest {
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    #[schemars(regex(pattern = "^[1-9][0-9]*$"), length(max = 19))]
    pub project_id: String,
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    #[schemars(regex(pattern = "^[1-9][0-9]*$"), length(max = 19))]
    pub user_id: String,
}

#[derive(Serialize, utoipa::ToSchema, schemars::JsonSchema)]
pub struct MemberChange {
    pub project_id: String,
    pub user_id: String,
    /// Null means the membership was removed.
    pub role: Option<Role>,
}

async fn apply(
    state: AppState,
    actor_id: i64,
    project_id: String,
    user_id: String,
    role: Option<Role>,
) -> Result<Json<MemberChange>, ApiError> {
    let input = ChangeMember {
        project_id: crate::invitations::id(&project_id)?,
        user_id: crate::invitations::id(&user_id)?,
        actor_id,
        role: role.map(|r| match r {
            Role::Owner => MemberRole::Owner,
            Role::Editor => MemberRole::Editor,
            Role::Viewer => MemberRole::Viewer,
        }),
    };
    let mut conn = connect(&state.database).await.map_err(database_error)?;
    match change_member(&mut conn, input)
        .await
        .map_err(database_error)?
    {
        MemberOutcome::Changed => Ok(Json(MemberChange {
            project_id,
            user_id,
            role,
        })),
        MemberOutcome::Forbidden => Err(ApiError(ErrorCode::Forbidden)),
        MemberOutcome::MemberNotFound => Err(ApiError(ErrorCode::MemberNotFound)),
        MemberOutcome::LastOwner => Err(ApiError(ErrorCode::LastOwner)),
    }
}

#[utoipa::path(
    post, path = "/api/memberships/role", operation_id = "changeMemberRole", request_body = ChangeRoleRequest,
    responses(
        (status = 200, description = "Member role changed (same role is a successful no-op)", body = MemberChange),
        (status = 400, description = "Invalid request", body = Problem),
        (status = 401, description = "Sign-in required", body = Problem),
        (status = 403, description = "Not a project owner or CSRF validation failed", body = Problem),
        (status = 404, description = "Member not found", body = Problem),
        (status = 409, description = "Last owner must remain", body = Problem),
        (status = 500, description = "Internal error", body = Problem),
        (status = 503, description = "Database busy", body = Problem)
    ), security(("BrowserSession" = []))
)]
pub async fn change_role_endpoint(
    State(state): State<AppState>,
    Actor(actor): Actor,
    body: Result<Json<ChangeRoleRequest>, JsonRejection>,
) -> Result<Json<MemberChange>, ApiError> {
    let Json(body) = body.map_err(|_| ApiError(ErrorCode::InvalidRequest))?;
    apply(state, actor, body.project_id, body.user_id, Some(body.role)).await
}

#[utoipa::path(
    post, path = "/api/memberships/remove", operation_id = "removeMember", request_body = RemoveMemberRequest,
    responses(
        (status = 200, description = "Membership removed", body = MemberChange),
        (status = 400, description = "Invalid request", body = Problem),
        (status = 401, description = "Sign-in required", body = Problem),
        (status = 403, description = "Not a project owner or CSRF validation failed", body = Problem),
        (status = 404, description = "Member not found", body = Problem),
        (status = 409, description = "Last owner must remain", body = Problem),
        (status = 500, description = "Internal error", body = Problem),
        (status = 503, description = "Database busy", body = Problem)
    ), security(("BrowserSession" = []))
)]
pub async fn remove_endpoint(
    State(state): State<AppState>,
    Actor(actor): Actor,
    body: Result<Json<RemoveMemberRequest>, JsonRejection>,
) -> Result<Json<MemberChange>, ApiError> {
    let Json(body) = body.map_err(|_| ApiError(ErrorCode::InvalidRequest))?;
    apply(state, actor, body.project_id, body.user_id, None).await
}
