//! Local development server: disposable data and allowlisted test identities
//! against the local OIDC issuer fixture. Not a production entry point.
use iris_reference::{
    app::{AppState, MIGRATOR, app, connect, seed, unix_time},
    identity::{Auth, store::Store},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() != Some("--local-oidc-demo") {
        return Err("requires --local-oidc-demo; disposable data and allowlisted test identities, not production".into());
    }
    let origin = std::env::var("IRIS_PUBLIC_ORIGIN")?;
    let issuer = std::env::var("IRIS_OIDC_ISSUER")?;
    let listen = std::env::var("IRIS_LISTEN").unwrap_or_else(|_| "127.0.0.1:3003".into());
    let dir = tempfile::tempdir()?;
    let database = dir.path().join("reference.db");
    let mut conn = connect(&database).await?;
    MIGRATOR.run(&mut conn).await?;
    seed(&mut conn, &issuer).await?;
    // Without invitations, a successful role change or removal needs a
    // non-owner member; the membership tests insert the same row.
    sqlx::query("INSERT INTO memberships (project_id, user_id, role) VALUES (41, 29, 'editor')")
        .execute(&mut conn)
        .await?;
    drop(conn);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&database)
                .foreign_keys(true)
                .busy_timeout(std::time::Duration::from_secs(1)),
        )
        .await?;
    let store = Store {
        pool,
        now: unix_time,
    };
    let auth = Auth::discover(store, origin, issuer, "iris-local".into(), None).await?;
    let app = app(auth).with_state(AppState {
        database,
        now: unix_time,
    });
    let listener = tokio::net::TcpListener::bind(&listen).await?;
    eprintln!("listening on http://{}", listener.local_addr()?);
    eprintln!("Iris reference application; disposable data, local test identities only.");
    axum::serve(listener, app).await?;
    Ok(())
}
