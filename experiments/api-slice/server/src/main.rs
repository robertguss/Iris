use iris_api_spike::{AppState, seed_demo, unix_time, utoipa_router};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() != Some("--dev-demo") {
        return Err(
            "Requires --dev-demo. This server uses synthetic identity, not authentication.".into(),
        );
    }
    // Every process gets fresh, disposable data; no shared/application DB writes.
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("demo.db");
    let mut conn = iris_sqlite_spike::connect(&database).await?;
    iris_sqlite_spike::migrate(&mut conn).await?;
    seed_demo(&mut conn, unix_time()).await?;
    drop(conn);
    let (router, api) = utoipa_router();
    let app = router
        .with_state(AppState {
            database,
            now: unix_time,
        })
        .route(
            "/api/openapi.json",
            axum::routing::get(move || async move { axum::Json(api) }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;
    eprintln!(
        "Iris API experiment: development identity enabled; disposable database; loopback only."
    );
    axum::serve(listener, app).await?;
    Ok(())
}
