use iris_api_spike::{
    AppState,
    auth::{Auth, store::Store},
    seed_demo, unix_time, utoipa_router,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() != Some("--local-oidc-demo") {
        return Err("requires --local-oidc-demo; disposable data and allowlisted test identities, not production".into());
    }
    let origin = std::env::var("IRIS_PUBLIC_ORIGIN")?;
    let issuer = std::env::var("IRIS_OIDC_ISSUER")?;
    let dir = tempfile::tempdir()?;
    let database = dir.path().join("auth.db");
    let mut conn = iris_sqlite_spike::connect(&database).await?;
    iris_sqlite_spike::migrate(&mut conn).await?;
    seed_demo(&mut conn, unix_time()).await?;
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&database)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(1));
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;
    let store = Store {
        pool,
        now: unix_time,
    };
    store.migrate().await?;
    for (sub, user) in [("alice", 11_i64), ("bob", 29)] {
        sqlx::query("INSERT INTO iris_external_identities VALUES(?,?,?)")
            .bind(&issuer)
            .bind(sub)
            .bind(user)
            .execute(&store.pool)
            .await?;
    }
    let mailer = iris_api_spike::delivery::Mailer::new(origin.clone(), 1025)?;
    let worker = tokio::spawn(mailer.run(database.clone()));
    let auth = Auth::discover(store.clone(), origin, issuer, "iris-local".into(), None).await?;
    let (router, api) = utoipa_router();
    let app = auth
        .layer(router)
        .with_state(AppState {
            database,
            now: unix_time,
        })
        .route(
            "/api/openapi.json",
            axum::routing::get(move || async move { axum::Json(api) }),
        );
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if store.cleanup().await.is_err() {
                eprintln!("auth cleanup failed");
            }
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3002").await?;
    eprintln!(
        "Iris local OIDC experiment listening; disposable data, no real-provider login configured."
    );
    tokio::select! {
        result = axum::serve(listener, app).into_future() => result?,
        _ = worker => return Err("delivery worker stopped".into()),
    }
    Ok(())
}
