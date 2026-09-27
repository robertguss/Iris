//! Application assembly: state, database access, schema, fixtures and routes.
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::Router;
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use utoipa::OpenApi;

use crate::identity::{self, Auth};

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

/// A fresh connection; callers drop it after failures rather than reuse it.
pub async fn connect(path: &Path) -> Result<SqliteConnection, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_millis(100));
    SqliteConnection::connect_with(&options).await
}

/// The application schema, including the session tables.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Disposable fixtures only; do not call this against an application database.
pub async fn seed(conn: &mut SqliteConnection, issuer: &str) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(
        "INSERT INTO users (id, display_name) VALUES (11, 'Alice Example'), (29, 'Bob Example');
        INSERT INTO projects (id, name) VALUES (41, 'Launch plan'), (43, 'Field notes');
        INSERT INTO memberships (project_id, user_id, role) VALUES (41, 11, 'owner'), (43, 29, 'owner')",
    )
    .execute(&mut *conn)
    .await?;
    for (subject, user_id) in [("alice", 11_i64), ("bob", 29)] {
        sqlx::query(
            "INSERT INTO iris_external_identities (issuer, subject, user_id) VALUES (?, ?, ?)",
        )
        .bind(issuer)
        .bind(subject)
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

#[derive(OpenApi)]
#[openapi(info(title = "Iris reference application", version = "0.1.0"))]
struct ApiDoc;

/// Collected routes and their document; assembly runs without an identity provider.
pub fn routes() -> (Router<AppState>, utoipa::openapi::OpenApi) {
    use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
    let (router, mut api) = utoipa_axum::router::OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes!(identity::session_info))
        .routes(utoipa_axum::routes!(identity::login))
        .routes(utoipa_axum::routes!(identity::callback))
        .routes(utoipa_axum::routes!(identity::logout))
        .split_for_parts();
    // No project license has been chosen; omit the inferred empty license.
    api.info.license = None;
    api.components
        .get_or_insert_with(Default::default)
        .add_security_scheme(
            "BrowserSession",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
                "__Host-iris-session",
                "Same-origin session and X-Iris-Csrf required. Explicit HTTP-loopback test mode uses iris-session-dev.",
            ))),
        );
    (router, api)
}

pub fn app(auth: Auth) -> Router<AppState> {
    auth.layer(routes().0)
}
