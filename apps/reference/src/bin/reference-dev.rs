//! Local development server: disposable data unless given a database path, and
//! allowlisted test identities against the local OIDC issuer fixture. Not a
//! production entry point.
use std::path::{Path, PathBuf};

use iris_reference::{
    app::{AppState, app, unix_time},
    identity::{Auth, store::Store},
    storage::Storage,
};

const USAGE: &str = "usage: reference-dev --local-oidc-demo [--database PATH]; \
requires --local-oidc-demo: disposable data unless --database is given, \
allowlisted test identities, not production";

#[tokio::main]
async fn main() -> std::process::ExitCode {
    // Reports errors by their messages rather than their debug form.
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("reference-dev: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The path is an argument, never an environment variable, so a runner that
    // passes its environment along cannot make a disposable run persistent.
    let database = match args.as_slice() {
        [flag] if flag == "--local-oidc-demo" => None,
        [flag, option, path] if flag == "--local-oidc-demo" && option == "--database" => {
            Some(PathBuf::from(path))
        }
        _ => return Err(USAGE.into()),
    };
    let origin = std::env::var("IRIS_PUBLIC_ORIGIN")?;
    let issuer = std::env::var("IRIS_OIDC_ISSUER")?;
    let listen = std::env::var("IRIS_LISTEN").unwrap_or_else(|_| "127.0.0.1:3003".into());
    let storage = match &database {
        Some(path) => Storage::open(path, &issuer).await?,
        None => Storage::disposable(&issuer).await?,
    };
    let data = match &database {
        Some(_) => format!("persistent data at {}", storage.path().display()),
        None => "disposable data".into(),
    };
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(storage.path())
                .foreign_keys(true)
                .busy_timeout(std::time::Duration::from_secs(1)),
        )
        .await?;
    let served = serve(pool.clone(), storage.path(), origin, issuer, &listen, &data).await;
    // Every connection closes before the ownership lock is released.
    pool.close().await;
    drop(storage);
    served
}

async fn serve(
    pool: sqlx::SqlitePool,
    database: &Path,
    origin: String,
    issuer: String,
    listen: &str,
    data: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = Store {
        pool,
        now: unix_time,
    };
    let auth = Auth::discover(store, origin, issuer, "iris-local".into(), None).await?;
    let app = app(auth).with_state(AppState {
        database: database.to_owned(),
        now: unix_time,
    });
    let listener = tokio::net::TcpListener::bind(listen).await?;
    eprintln!("Iris reference application; {data}, local test identities only.");
    eprintln!("listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
