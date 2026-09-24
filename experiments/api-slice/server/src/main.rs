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
    let mailer =
        iris_api_spike::delivery::Mailer::new(std::env::var("IRIS_INVITATION_ORIGIN")?, 1025)?;
    let worker = tokio::spawn(mailer.run(database.clone()));
    let (router, api) = utoipa_router();
    let app = iris_api_spike::development_identity(router)
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
    tokio::select! {
        result = axum::serve(listener, app).into_future() => result?,
        _ = worker => return Err("delivery worker stopped".into()),
    }
    Ok(())
}
