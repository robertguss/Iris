//! HTTP adapter for membership operations, ported from the S16 experiment.
use super::{
    Collected, Mapping, Operation, Recovery, RequestId, Shared, boundary, parse_id, shared,
    success_schemas,
};
use crate::{
    app::AppState,
    domains::memberships as action,
    identity::{Actor, ApiError},
};
use action::{Acknowledged, ActionError, FailureKind, Rejection, StopReason};
use axum::{
    Extension, Json,
    extract::{State, rejection::JsonRejection},
    http::Method,
    response::Response,
};
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;
use strum::VariantArray;
use utoipa_axum::router::OpenApiRouter;

#[derive(Clone, Copy, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Editor,
    Viewer,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangeRoleRequest {
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    pub project_id: String,
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    pub user_id: String,
    pub role: Role,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveMemberRequest {
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    pub project_id: String,
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    pub user_id: String,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
enum Completion {
    Acknowledged,
}
#[derive(Serialize, utoipa::ToSchema)]
struct ChangeRoleSuccess {
    completion: Completion,
}
#[derive(Serialize, utoipa::ToSchema)]
struct RemoveMemberSuccess {
    completion: Completion,
}
fn change_role_success(_: Acknowledged) -> ChangeRoleSuccess {
    ChangeRoleSuccess {
        completion: Completion::Acknowledged,
    }
}
fn remove_member_success(_: Acknowledged) -> RemoveMemberSuccess {
    RemoveMemberSuccess {
        completion: Completion::Acknowledged,
    }
}

/// Both operations permit the same rejections, so they share one mapping.
fn membership_rejection(r: Rejection) -> Mapping {
    let status = match r {
        Rejection::Forbidden => 403,
        Rejection::MemberNotFound => 404,
        Rejection::LastOwner => 409,
    };
    let descriptor = r.descriptor();
    Mapping {
        status,
        kind: "rejected",
        code: Some(descriptor.code),
        message: descriptor.summary,
        rule: descriptor.rule,
        prerequisite: descriptor.prerequisite,
    }
}

const ACKNOWLEDGED: Mapping = Mapping {
    status: 200,
    kind: "success",
    code: None,
    message: "Commit acknowledged",
    rule: None,
    prerequisite: None,
};

const RESUBMIT_ONLY: Recovery = Recovery {
    inspect: false,
    read: false,
    replay: false,
    new_submission: "current authority and intent required",
};

pub(crate) static CHANGE_ROLE: Operation<Rejection> = Operation {
    name: "memberships.change_role",
    public_id: "changeMemberRole",
    handler: "change_role_endpoint",
    method: Method::POST,
    success: ACKNOWLEDGED,
    success_schema: "ChangeRoleSuccess",
    rejections: Rejection::VARIANTS,
    rejection: membership_rejection,
    recovery: RESUBMIT_ONLY,
};

pub(crate) static REMOVE_MEMBER: Operation<Rejection> = Operation {
    name: "memberships.remove_member",
    public_id: "removeMember",
    handler: "remove_member_endpoint",
    method: Method::POST,
    success: ACKNOWLEDGED,
    success_schema: "RemoveMemberSuccess",
    rejections: Rejection::VARIANTS,
    rejection: membership_rejection,
    recovery: RESUBMIT_ONLY,
};

fn reply<S: Serialize>(
    op: &Operation<Rejection>,
    id: &RequestId,
    result: Result<Acknowledged, ActionError>,
    project: fn(Acknowledged) -> S,
) -> Response {
    match result {
        Ok(ack) => op.render(
            &op.success,
            id,
            Some(serde_json::to_value(project(ack)).unwrap()),
        ),
        Err(ActionError::Rejected(r)) => op.render(&(op.rejection)(r), id, None),
        Err(ActionError::Failed {
            primary:
                StopReason::Execution {
                    kind: FailureKind::Busy,
                    ..
                },
            ..
        }) => op.render(&shared(Shared::Unavailable), id, None),
        Err(ActionError::Failed { .. }) => op.render(&shared(Shared::Internal), id, None),
    }
}

/// A fresh connection, or the shared failure for not opening one.
async fn open(state: &AppState) -> Result<SqliteConnection, Shared> {
    crate::app::connect(&state.database).await.map_err(|e| {
        if crate::app::is_busy(&e) {
            Shared::Unavailable
        } else {
            Shared::Internal
        }
    })
}

#[utoipa::path(post, path = "/api/memberships/role", request_body = ChangeRoleRequest, security(("BrowserSession" = [])))]
async fn change_role_endpoint(
    State(state): State<AppState>,
    Extension(id): Extension<RequestId>,
    actor: Result<Actor, ApiError>,
    body: Result<Json<ChangeRoleRequest>, JsonRejection>,
) -> Response {
    let op = &CHANGE_ROLE;
    let Ok(actor) = actor else {
        return op.render(&shared(Shared::Unauthenticated), &id, None);
    };
    let Ok(Json(body)) = body else {
        return op.render(&shared(Shared::Invalid), &id, None);
    };
    let (Ok(project_id), Ok(user_id)) = (parse_id(&body.project_id), parse_id(&body.user_id))
    else {
        return op.render(&shared(Shared::Invalid), &id, None);
    };
    let input = action::ChangeRole {
        project_id,
        user_id,
        role: match body.role {
            Role::Owner => action::MemberRole::Owner,
            Role::Editor => action::MemberRole::Editor,
            Role::Viewer => action::MemberRole::Viewer,
        },
    };
    let mut conn = match open(&state).await {
        Ok(conn) => conn,
        Err(failure) => return op.render(&shared(failure), &id, None),
    };
    let result = action::change_role(&mut conn, &actor, input).await;
    reply(op, &id, result, change_role_success)
}

#[utoipa::path(post, path = "/api/memberships/remove", request_body = RemoveMemberRequest, security(("BrowserSession" = [])))]
async fn remove_member_endpoint(
    State(state): State<AppState>,
    Extension(id): Extension<RequestId>,
    actor: Result<Actor, ApiError>,
    body: Result<Json<RemoveMemberRequest>, JsonRejection>,
) -> Response {
    let op = &REMOVE_MEMBER;
    let Ok(actor) = actor else {
        return op.render(&shared(Shared::Unauthenticated), &id, None);
    };
    let Ok(Json(body)) = body else {
        return op.render(&shared(Shared::Invalid), &id, None);
    };
    let (Ok(project_id), Ok(user_id)) = (parse_id(&body.project_id), parse_id(&body.user_id))
    else {
        return op.render(&shared(Shared::Invalid), &id, None);
    };
    let input = action::RemoveMember {
        project_id,
        user_id,
    };
    let mut conn = match open(&state).await {
        Ok(conn) => conn,
        Err(failure) => return op.render(&shared(failure), &id, None),
    };
    let result = action::remove_member(&mut conn, &actor, input).await;
    reply(op, &id, result, remove_member_success)
}

/// Explicit ecosystem registration: each operation is collected alone.
pub fn change_role() -> Collected {
    super::collect(
        &CHANGE_ROLE,
        OpenApiRouter::new().routes(utoipa_axum::routes!(change_role_endpoint)),
        success_schemas::<ChangeRoleSuccess>(),
        |router, auth| boundary(auth.layer(router), &CHANGE_ROLE),
    )
}

pub fn remove_member() -> Collected {
    super::collect(
        &REMOVE_MEMBER,
        OpenApiRouter::new().routes(utoipa_axum::routes!(remove_member_endpoint)),
        success_schemas::<RemoveMemberSuccess>(),
        |router, auth| boundary(auth.layer(router), &REMOVE_MEMBER),
    )
}

pub fn collect() -> Vec<Collected> {
    vec![change_role(), remove_member()]
}

#[cfg(test)]
mod tests;
