//! HTTP adapter for project reads.
use super::{
    Mount, PAGE_READ, PageQuery, classify, decode_cursor, encode_cursor, memberships::Role, open,
    parse_limit, read_reply,
};
use crate::{
    app::AppState,
    domains::projects as action,
    identity::{Actor, ApiError},
    read::Page,
};
use action::ListMineRejection;
use axum::{
    Extension, Router,
    extract::{Query, State, rejection::QueryRejection},
    http::Method,
    response::Response,
};
use iris::{Collected, Mapping, Operation, RequestId, Shared, shared, success_schemas};
use serde::Serialize;
use utoipa_axum::router::OpenApiRouter;

/// The caller's own membership in one project.
#[derive(Serialize, utoipa::ToSchema)]
struct ProjectSummary {
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    project_id: String,
    name: String,
    role: Role,
}

/// Present state when read, not a snapshot of the collection.
#[derive(Serialize, utoipa::ToSchema)]
struct ProjectPage {
    items: Vec<ProjectSummary>,
    /// Null when no further projects existed when this page was read.
    #[schema(required = true)]
    next_cursor: Option<String>,
}

fn project_page(page: Page<action::ProjectSummary>) -> ProjectPage {
    ProjectPage {
        items: page
            .items
            .into_iter()
            .map(|project| ProjectSummary {
                project_id: project.project_id.to_string(),
                name: project.name,
                role: project.role.into(),
            })
            .collect(),
        next_cursor: page.next.map(encode_cursor),
    }
}

fn no_rejection(r: ListMineRejection) -> Mapping {
    match r {}
}

pub(crate) static LIST_MINE: Operation<ListMineRejection> = Operation {
    name: "projects.list_mine",
    public_id: "listMyProjects",
    handler: "list_mine_endpoint",
    method: Method::GET,
    success: PAGE_READ,
    success_schema: "ProjectPage",
    rejections: &[],
    rejection: no_rejection,
    recovery: None,
};

// Authentication first, then the query, then the read.
#[utoipa::path(get, path = "/api/projects", params(PageQuery), security(("BrowserSession" = [])))]
async fn list_mine_endpoint(
    State(state): State<AppState>,
    Extension(id): Extension<RequestId>,
    actor: Result<Actor, ApiError>,
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Response {
    let op = &LIST_MINE;
    let Ok(actor) = actor else {
        return op.render(&shared(Shared::Unauthenticated), &id, None);
    };
    let Ok(Query(query)) = query else {
        return op.render(&shared(Shared::Invalid), &id, None);
    };
    let (Ok(limit), Ok(after)) = (
        parse_limit(query.limit.as_deref()),
        query.cursor.as_deref().map(decode_cursor).transpose(),
    ) else {
        return op.render(&shared(Shared::Invalid), &id, None);
    };
    let conn = match open(&state).await {
        Ok(conn) => conn,
        Err(failure) => return op.render(&shared(failure), &id, None),
    };
    let result = action::list_mine(conn, &actor, action::ListMine { limit, after }).await;
    read_reply(op, &id, result.map(project_page))
}

/// Explicit ecosystem registration: the operation is collected alone.
pub fn list_mine() -> Collected<AppState> {
    iris::collect(
        &LIST_MINE,
        OpenApiRouter::new().routes(utoipa_axum::routes!(list_mine_endpoint)),
        success_schemas::<ProjectPage>(),
    )
}

fn mount_list_mine(router: Router<AppState>, auth: crate::identity::Auth) -> Router<AppState> {
    iris::boundary(auth.layer(router), &LIST_MINE, classify)
}

pub(crate) fn collect() -> Vec<(Collected<AppState>, Mount)> {
    vec![(list_mine(), mount_list_mine)]
}

#[cfg(test)]
mod tests;
