//! Application assembly: state, database access, schema, fixtures and routes.
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{Router, http::StatusCode};
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use utoipa::OpenApi;

use crate::{
    http,
    identity::{self, Auth},
};

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

/// SQLite reports busy as primary result code 5, possibly extended.
pub fn is_busy(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|e| e.code())
        .and_then(|code| code.parse::<i32>().ok())
        .is_some_and(|code| code & 0xff == 5)
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

/// The session endpoints, collected once for both the router and the document.
fn collect_identity() -> (Router<AppState>, utoipa::openapi::OpenApi) {
    utoipa_axum::router::OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes!(identity::session_info))
        .routes(utoipa_axum::routes!(identity::login))
        .routes(utoipa_axum::routes!(identity::callback))
        .routes(utoipa_axum::routes!(identity::logout))
        .split_for_parts()
}

struct Assembled {
    identity: Router<AppState>,
    operations: Vec<(Router<AppState>, http::Mount)>,
    api: utoipa::openapi::OpenApi,
}

/// The one checked assembly behind both the application and its export.
/// Panics when a bridge, component, code or identifier is inconsistent.
fn assemble() -> Assembled {
    let (identity, mut api) = collect_identity();
    // No project license has been chosen; omit the inferred empty license.
    api.info.license = None;
    api.components
        .get_or_insert_with(Default::default)
        .add_security_scheme("BrowserSession", identity::security_scheme());
    let mut operations = Vec::new();
    let mut entries = Vec::new();
    for (collected, mount) in http::memberships::collect()
        .into_iter()
        .chain(http::projects::collect())
    {
        iris::merge_checked(&mut api, collected.api);
        entries.push(collected.entry);
        operations.push((collected.router, mount));
    }
    iris::check_catalog(&api, &entries);
    Assembled {
        identity,
        operations,
        api,
    }
}

/// The application document; assembly runs without an identity provider.
pub fn openapi() -> utoipa::openapi::OpenApi {
    assemble().api
}

/// Each domain route keeps its envelope boundary outside its session layer.
/// Unmatched routes fall back to a plain 404 outside every layer.
pub fn app(auth: Auth) -> Router<AppState> {
    let assembled = assemble();
    assembled
        .operations
        .into_iter()
        .fold(
            auth.clone().layer(assembled.identity),
            |router, (operation, mount)| router.merge(mount(operation, auth.clone())),
        )
        .fallback(|| async { StatusCode::NOT_FOUND })
}
