//! HTTP adapter for membership operations, ported from the S16 experiment.
use super::{
    Mount, PAGE_READ, PageQuery, classify, decode_cursor, encode_cursor, open, parse_id,
    parse_limit, read_reply,
};
use crate::{
    app::AppState,
    domains::memberships as action,
    identity::{Actor, ApiError},
    read::Page,
};
use action::{
    Acknowledged, ActionError, Descriptor, FailureKind, ListMembersRejection, MemberRole,
    Rejection, StopReason,
};
use axum::{
    Extension, Json, Router,
    extract::{
        Path, Query, State,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::Method,
    response::Response,
};
use iris::{
    Collected, CurrentStateRead, Mapping, Operation, Recovery, RequestId, Shared, shared,
    success_schemas,
};
use serde::{Deserialize, Serialize};
use strum::VariantArray;
use utoipa_axum::router::OpenApiRouter;

#[derive(Clone, Copy, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Editor,
    Viewer,
}

impl From<MemberRole> for Role {
    fn from(role: MemberRole) -> Self {
        match role {
            MemberRole::Owner => Self::Owner,
            MemberRole::Editor => Self::Editor,
            MemberRole::Viewer => Self::Viewer,
        }
    }
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

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Path)]
struct ProjectPath {
    #[param(pattern = "^[1-9][0-9]*$", max_length = 19)]
    project_id: String,
}

/// Never includes contact details.
#[derive(Serialize, utoipa::ToSchema)]
struct MemberSummary {
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    user_id: String,
    display_name: String,
    role: Role,
}

/// Present state when read, not a snapshot of the collection.
#[derive(Serialize, utoipa::ToSchema)]
struct MemberPage {
    items: Vec<MemberSummary>,
    /// Null when no further members existed when this page was read.
    #[schema(required = true)]
    next_cursor: Option<String>,
}

fn member_page(page: Page<action::MemberSummary>) -> MemberPage {
    MemberPage {
        items: page
            .items
            .into_iter()
            .map(|member| MemberSummary {
                user_id: member.user_id.to_string(),
                display_name: member.display_name,
                role: member.role.into(),
            })
            .collect(),
        next_cursor: page.next.map(encode_cursor),
    }
}

fn rejected(status: u16, descriptor: Descriptor) -> Mapping {
    Mapping {
        status,
        kind: "rejected",
        code: Some(descriptor.code),
        message: descriptor.summary,
        rule: descriptor.rule,
        prerequisite: descriptor.prerequisite,
    }
}

/// Both operations permit the same rejections, so they share one mapping.
fn membership_rejection(r: Rejection) -> Mapping {
    let status = match r {
        Rejection::Forbidden => 403,
        Rejection::MemberNotFound => 404,
        Rejection::LastOwner => 409,
    };
    rejected(status, r.descriptor())
}

/// An unknown project and a non-member get the same refusal.
fn list_members_rejection(r: ListMembersRejection) -> Mapping {
    let status = match r {
        ListMembersRejection::Forbidden => 403,
    };
    rejected(status, r.descriptor())
}

const ACKNOWLEDGED: Mapping = Mapping {
    status: 200,
    kind: "success",
    code: None,
    message: "Commit acknowledged",
    rule: None,
    prerequisite: None,
};

/// No inspection or replay. After an unresolved attempt, the attempt's project
/// can be read again: its members as they are when read, under the caller's
/// current authorization. A matching role, an absent member or a refused read
/// resolves nothing about the attempt (S14).
const MEMBER_LIST_READ: Recovery = Recovery {
    inspect: false,
    read: Some(CurrentStateRead {
        operation: "listProjectMembers",
        path_inputs: &[("project_id", "project_id")],
    }),
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
    recovery: Some(MEMBER_LIST_READ),
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
    recovery: Some(MEMBER_LIST_READ),
};

pub(crate) static LIST_MEMBERS: Operation<ListMembersRejection> = Operation {
    name: "memberships.list",
    public_id: "listProjectMembers",
    handler: "list_members_endpoint",
    method: Method::GET,
    success: PAGE_READ,
    success_schema: "MemberPage",
    rejections: ListMembersRejection::VARIANTS,
    rejection: list_members_rejection,
    recovery: None,
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

// Authentication first, then the path and query, then the read.
#[utoipa::path(get, path = "/api/projects/{project_id}/members", params(ProjectPath, PageQuery), security(("BrowserSession" = [])))]
async fn list_members_endpoint(
    State(state): State<AppState>,
    Extension(id): Extension<RequestId>,
    actor: Result<Actor, ApiError>,
    path: Result<Path<ProjectPath>, PathRejection>,
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Response {
    let op = &LIST_MEMBERS;
    let Ok(actor) = actor else {
        return op.render(&shared(Shared::Unauthenticated), &id, None);
    };
    let (Ok(Path(path)), Ok(Query(query))) = (path, query) else {
        return op.render(&shared(Shared::Invalid), &id, None);
    };
    let (Ok(project_id), Ok(limit), Ok(after)) = (
        parse_id(&path.project_id),
        parse_limit(query.limit.as_deref()),
        query.cursor.as_deref().map(decode_cursor).transpose(),
    ) else {
        return op.render(&shared(Shared::Invalid), &id, None);
    };
    let conn = match open(&state).await {
        Ok(conn) => conn,
        Err(failure) => return op.render(&shared(failure), &id, None),
    };
    let input = action::ListMembers {
        project_id,
        limit,
        after,
    };
    let result = action::list_members(conn, &actor, input).await;
    read_reply(op, &id, result.map(member_page))
}

/// Explicit ecosystem registration: each operation is collected alone.
pub fn change_role() -> Collected<AppState> {
    iris::collect(
        &CHANGE_ROLE,
        OpenApiRouter::new().routes(utoipa_axum::routes!(change_role_endpoint)),
        success_schemas::<ChangeRoleSuccess>(),
    )
}

pub fn remove_member() -> Collected<AppState> {
    iris::collect(
        &REMOVE_MEMBER,
        OpenApiRouter::new().routes(utoipa_axum::routes!(remove_member_endpoint)),
        success_schemas::<RemoveMemberSuccess>(),
    )
}

pub fn list_members() -> Collected<AppState> {
    iris::collect(
        &LIST_MEMBERS,
        OpenApiRouter::new().routes(utoipa_axum::routes!(list_members_endpoint)),
        success_schemas::<MemberPage>(),
    )
}

fn mount_change_role(router: Router<AppState>, auth: crate::identity::Auth) -> Router<AppState> {
    iris::boundary(auth.layer(router), &CHANGE_ROLE, classify)
}

fn mount_remove_member(router: Router<AppState>, auth: crate::identity::Auth) -> Router<AppState> {
    iris::boundary(auth.layer(router), &REMOVE_MEMBER, classify)
}

fn mount_list_members(router: Router<AppState>, auth: crate::identity::Auth) -> Router<AppState> {
    iris::boundary(auth.layer(router), &LIST_MEMBERS, classify)
}

/// Each collected operation with the mount that layers its session stack
/// and boundary.
pub(crate) fn collect() -> Vec<(Collected<AppState>, Mount)> {
    vec![
        (change_role(), mount_change_role),
        (remove_member(), mount_remove_member),
        (list_members(), mount_list_members),
    ]
}

#[cfg(test)]
mod caller_loss;
#[cfg(test)]
pub(crate) mod list_tests;
#[cfg(test)]
pub(crate) mod tests;
