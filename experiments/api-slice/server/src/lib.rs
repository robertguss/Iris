use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    extract::{FromRequestParts, State, rejection::JsonRejection},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use iris_sqlite_spike::{AcceptInvitation, Outcome, accept, connect, is_busy};
use serde::{Deserialize, Serialize};
use utoipa::OpenApi;

pub const ACCEPT_PATH: &str = "/api/invitations/accept";
pub const ISSUE_PATH: &str = "/api/invitations";
pub mod auth;
pub mod delivery;
mod invitations;
use invitations::{IssuedInvitation, issue_endpoint};

#[derive(Clone)]
pub struct AppState {
    pub database: PathBuf,
    pub now: fn() -> i64,
}

pub fn unix_time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_secs() as i64
}

#[derive(Deserialize, utoipa::ToSchema, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptRequest {
    /// Opaque invitation credential. Identity is never accepted in this body.
    #[schema(min_length = 1, max_length = 256)]
    #[schemars(length(min = 1, max = 256))]
    pub token: String,
}

#[derive(Serialize, utoipa::ToSchema, schemars::JsonSchema)]
pub struct Acceptance {
    /// IDs are strings at the wire boundary to avoid JavaScript integer loss.
    pub project_id: String,
    pub user_id: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    Csrf,
    LoginFailed,
    Forbidden,
    RecipientNotFound,
    AlreadyMember,
    InvitationPending,
    Unauthorized,
    NotFound,
    Expired,
    AlreadyAccepted,
    Internal,
    Unavailable,
}

#[derive(Serialize, utoipa::ToSchema, schemars::JsonSchema)]
pub struct Problem {
    pub code: ErrorCode,
    pub message: String,
}

pub struct ApiError(pub ErrorCode);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            ErrorCode::Csrf => (
                StatusCode::FORBIDDEN,
                "Refresh the session and send a valid same-origin CSRF token.",
            ),
            ErrorCode::LoginFailed => (
                StatusCode::UNAUTHORIZED,
                "Login failed. Start a new login attempt.",
            ),
            ErrorCode::InvalidRequest => (
                StatusCode::BAD_REQUEST,
                "Provide the required fields with valid values and no extra fields.",
            ),
            ErrorCode::Forbidden => (
                StatusCode::FORBIDDEN,
                "Only a project owner can issue invitations.",
            ),
            ErrorCode::RecipientNotFound => (StatusCode::NOT_FOUND, "Recipient not found."),
            ErrorCode::AlreadyMember => {
                (StatusCode::CONFLICT, "The recipient is already a member.")
            }
            ErrorCode::InvitationPending => (
                StatusCode::CONFLICT,
                "An unexpired invitation already exists.",
            ),
            ErrorCode::Unauthorized => (StatusCode::UNAUTHORIZED, "Sign in to continue."),
            ErrorCode::NotFound => (
                StatusCode::NOT_FOUND,
                "Invitation not found for this identity.",
            ),
            ErrorCode::Expired => (StatusCode::CONFLICT, "This invitation has expired."),
            ErrorCode::AlreadyAccepted => (
                StatusCode::CONFLICT,
                "This invitation has already been accepted.",
            ),
            ErrorCode::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "The request could not be completed.",
            ),
            ErrorCode::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "The database is busy. Try again shortly.",
            ),
        };
        (
            status,
            Json(Problem {
                code: self.0,
                message: message.into(),
            }),
        )
            .into_response()
    }
}

/// Only explicitly assembled middleware can supply an actor; headers cannot.
#[derive(Clone)]
pub struct Actor(i64);

impl<S: Send + Sync> FromRequestParts<S> for Actor {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Actor>()
            .cloned()
            .ok_or(ApiError(ErrorCode::Unauthorized))
    }
}

/// Isolated legacy demo adapter. Never applied by the session-authenticated app.
#[cfg(feature = "dev-identity")]
pub fn development_identity(router: Router<AppState>) -> Router<AppState> {
    router.layer(axum::middleware::from_fn(
        async |mut req: axum::extract::Request, next: axum::middleware::Next| {
            let mut values = req.headers().get_all("x-iris-dev-user").iter();
            let id = match (values.next().and_then(|v| v.to_str().ok()), values.next()) {
                (Some("11"), None) => Some(11),
                (Some("29"), None) => Some(29),
                _ => None,
            };
            if let Some(id) = id {
                req.extensions_mut().insert(Actor(id));
            }
            next.run(req).await
        },
    ))
}

impl aide::OperationInput for Actor {
    fn operation_input(_: &mut aide::generate::GenContext, _: &mut aide::openapi::Operation) {}
}

impl aide::OperationOutput for ApiError {
    type Inner = Problem;
}

#[utoipa::path(
    post, path = "/api/invitations/accept", operation_id = "acceptInvitation",
    request_body = AcceptRequest,
    responses(
        (status = 200, description = "Invitation accepted; existing membership role preserved", body = Acceptance),
        (status = 400, description = "Invalid JSON, fields, or token length", body = Problem),
        (status = 401, description = "Sign-in required", body = Problem),
        (status = 403, description = "CSRF validation failed", body = Problem),
        (status = 404, description = "Unknown token or wrong recipient", body = Problem),
        (status = 409, description = "Expired or already accepted", body = Problem),
        (status = 500, description = "Internal error", body = Problem),
        (status = 503, description = "Database busy", body = Problem)
    ),
    security(("BrowserSession" = []))
)]
async fn accept_endpoint(
    State(state): State<AppState>,
    Actor(user_id): Actor,
    body: Result<Json<AcceptRequest>, JsonRejection>,
) -> Result<Json<Acceptance>, ApiError> {
    let Json(body) = body.map_err(|_| ApiError(ErrorCode::InvalidRequest))?;
    if !(1..=256).contains(&body.token.chars().count()) {
        return Err(ApiError(ErrorCode::InvalidRequest));
    }
    let mut conn = connect(&state.database).await.map_err(database_error)?;
    let outcome = accept(
        &mut conn,
        AcceptInvitation {
            token: &body.token,
            user_id,
            now: (state.now)(),
        },
    )
    .await
    .map_err(database_error)?;
    match outcome {
        Outcome::Accepted { project_id } => Ok(Json(Acceptance {
            project_id: project_id.to_string(),
            user_id: user_id.to_string(),
        })),
        Outcome::NotFound => Err(ApiError(ErrorCode::NotFound)),
        Outcome::Expired => Err(ApiError(ErrorCode::Expired)),
        Outcome::AlreadyAccepted => Err(ApiError(ErrorCode::AlreadyAccepted)),
    }
}

fn database_error(error: sqlx::Error) -> ApiError {
    if is_busy(&error) {
        ApiError(ErrorCode::Unavailable)
    } else {
        ApiError(ErrorCode::Internal)
    }
}

#[derive(OpenApi)]
#[openapi(info(title = "Iris invitation experiment", version = "0.1.0"))]
struct ApiDoc;

pub fn utoipa_router() -> (Router<AppState>, utoipa::openapi::OpenApi) {
    use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
    let (router, mut api) = utoipa_axum::router::OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes!(accept_endpoint))
        .routes(utoipa_axum::routes!(invitations::issue_endpoint))
        .routes(utoipa_axum::routes!(auth::session_info))
        .routes(utoipa_axum::routes!(auth::login))
        .routes(utoipa_axum::routes!(auth::callback))
        .routes(utoipa_axum::routes!(auth::logout))
        .split_for_parts();
    // No project license has been chosen; omit the inferred empty license.
    api.info.license = None;
    api.components
        .get_or_insert_with(Default::default)
        .add_security_scheme(
            "BrowserSession",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("__Host-iris-session"))),
        );
    (router, api)
}

pub fn aide_router() -> (Router<AppState>, aide::openapi::OpenApi) {
    use aide::{
        axum::{ApiRouter, routing::post_with},
        openapi::{ApiKeyLocation, SecurityScheme},
    };
    // Responses are declared explicitly for an apples-to-apples comparison.
    aide::generate::infer_responses(false);
    let app = ApiRouter::new().api_route(
        ACCEPT_PATH,
        post_with(accept_endpoint, |op| {
            op.id("acceptInvitation")
                .security_requirement("BrowserSession")
                .response_with::<200, Json<Acceptance>, _>(|r| {
                    r.description("Invitation accepted; existing membership role preserved")
                })
                .response_with::<400, Json<Problem>, _>(|r| {
                    r.description("Invalid JSON, fields, or token length")
                })
                .response_with::<401, Json<Problem>, _>(|r| r.description("Sign-in required"))
                .response_with::<403, Json<Problem>, _>(|r| r.description("CSRF validation failed"))
                .response_with::<404, Json<Problem>, _>(|r| {
                    r.description("Unknown token or wrong recipient")
                })
                .response_with::<409, Json<Problem>, _>(|r| {
                    r.description("Expired or already accepted")
                })
                .response_with::<500, Json<Problem>, _>(|r| r.description("Internal error"))
                .response_with::<503, Json<Problem>, _>(|r| r.description("Database busy"))
        }),
    );
    let app = app.api_route(
        ISSUE_PATH,
        post_with(issue_endpoint, |op| {
            op.id("issueInvitation")
                .security_requirement("BrowserSession")
                .response_with::<201, Json<IssuedInvitation>, _>(|r| {
                    r.description("Invitation issued; membership unchanged")
                })
                .response_with::<400, Json<Problem>, _>(|r| r.description("Invalid request"))
                .response_with::<401, Json<Problem>, _>(|r| r.description("Sign-in required"))
                .response_with::<403, Json<Problem>, _>(|r| {
                    r.description("Not a project owner or CSRF validation failed")
                })
                .response_with::<404, Json<Problem>, _>(|r| r.description("Recipient not found"))
                .response_with::<409, Json<Problem>, _>(|r| {
                    r.description("Already a member or invitation pending")
                })
                .response_with::<500, Json<Problem>, _>(|r| r.description("Internal error"))
                .response_with::<503, Json<Problem>, _>(|r| r.description("Database busy"))
        }),
    );
    let mut api = aide::openapi::OpenApi::default();
    let router = app.finish_api_with(&mut api, |api| {
        api.title("Iris invitation experiment")
            .version("0.1.0")
            .security_scheme(
            "BrowserSession",
            SecurityScheme::ApiKey {
                location: ApiKeyLocation::Cookie,
                name: "__Host-iris-session".into(),
                description: Some(
                    "HTTPS browser session; unsafe requests also require Origin and X-Iris-Csrf."
                        .into(),
                ),
                extensions: Default::default(),
            },
        )
    });
    (router, api)
}

/// Disposable fixtures only; do not call this against an application database.
pub async fn seed_demo(conn: &mut sqlx::SqliteConnection, now: i64) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(
        "INSERT INTO users VALUES (11), (29); INSERT INTO projects VALUES (7), (19), (41), (43);
        INSERT INTO user_contacts VALUES (11, 'alice@example.test'), (29, 'bob@example.test');
        INSERT INTO memberships VALUES (41, 11, 'owner'), (43, 29, 'owner')",
    )
    .execute(&mut *conn)
    .await?;
    for (token, project, recipient, expiry) in [
        ("iris-valid", 7_i64, 11_i64, now + 3600),
        ("iris-expired", 7, 11, now),
        ("iris-bob", 19, 29, now + 3600),
    ] {
        sqlx::query("INSERT INTO invitations (token_hash, project_id, recipient_id, role, expires_at) VALUES (?, ?, ?, 'editor', ?)")
            .bind(iris_sqlite_spike::token_hash(token)).bind(project).bind(recipient).bind(expiry)
            .execute(&mut *conn).await?;
    }
    Ok(())
}
