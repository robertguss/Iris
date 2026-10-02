//! Local development server: disposable data unless given a database path, and
//! allowlisted test identities against the local OIDC issuer fixture. With
//! `--reset` it replaces that database with a newly seeded one and exits. On
//! SIGINT or SIGTERM it drains and closes its connections within fixed
//! deadlines (`lifecycle`). Not a production entry point.
use std::path::{Path, PathBuf};

use iris_reference::{
    app::{AppState, app, unix_time},
    identity::{Auth, store::Store},
    lifecycle::{self, Connections, Process, Stopped},
    storage::{Storage, reset_command},
};

const USAGE: &str = "usage: reference-dev --local-oidc-demo [--database PATH [--reset]]; \
requires --local-oidc-demo: disposable data unless --database is given, \
allowlisted test identities, not production; --reset replaces the database \
at PATH with a newly seeded one and exits";

/// A server that has stopped serving, with what `lifecycle::finish` ends.
struct Served {
    storage: Storage,
    pool: sqlx::SqlitePool,
    connections: Connections,
    stopped: Stopped,
}

fn main() -> std::process::ExitCode {
    // The runtime is built by hand: `lifecycle::finish` takes it by value.
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => return refuse(&error),
    };
    match runtime.block_on(run()) {
        Ok(None) => std::process::ExitCode::SUCCESS,
        Ok(Some(served)) => lifecycle::finish(
            Process {
                runtime,
                storage: served.storage,
                pool: served.pool,
                connections: served.connections,
            },
            served.stopped,
            lifecycle::CLOSE,
        ),
        Err(error) => refuse(&*error),
    }
}

/// Reports an error by its message rather than its debug form.
fn refuse(error: &dyn std::error::Error) -> std::process::ExitCode {
    eprintln!("reference-dev: {error}");
    std::process::ExitCode::FAILURE
}

/// `None` after a reset; otherwise the server once it has stopped serving.
async fn run() -> Result<Option<Served>, Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The path is an argument, never an environment variable, so a runner that
    // passes its environment along cannot make a disposable run persistent.
    let database = match args.as_slice() {
        [flag] if flag == "--local-oidc-demo" => None,
        [flag, option, path] if flag == "--local-oidc-demo" && option == "--database" => {
            Some(PathBuf::from(path))
        }
        [flag, option, path, reset]
            if flag == "--local-oidc-demo" && option == "--database" && reset == "--reset" =>
        {
            // The new database's identities are bound to the issuer; nothing
            // else of the server's configuration is read, and nothing listens.
            let issuer = std::env::var("IRIS_OIDC_ISSUER")?;
            let storage = Storage::reset(Path::new(path), &issuer).await?;
            eprintln!(
                "reference-dev: reset database at {}",
                storage.path().display()
            );
            return Ok(None);
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
    let connections = Connections::default();
    let served = serve(
        pool.clone(),
        connections.clone(),
        storage.path(),
        origin,
        issuer,
        &listen,
        &data,
    )
    .await;
    match served {
        Ok(stopped) => Ok(Some(Served {
            storage,
            pool,
            connections,
            stopped,
        })),
        Err(error) => {
            // Nothing was served: the pool's are the only connections, and
            // they close before the ownership lock is released.
            pool.close().await;
            drop(storage);
            Err(error)
        }
    }
}

/// Starts serving, and returns how serving ended; an error means it never
/// began.
async fn serve(
    pool: sqlx::SqlitePool,
    connections: Connections,
    database: &Path,
    origin: String,
    issuer: String,
    listen: &str,
    data: &str,
) -> Result<Stopped, Box<dyn std::error::Error>> {
    check_identity_issuer(&pool, database, &issuer).await?;
    let store = Store {
        pool,
        now: unix_time,
    };
    let auth = Auth::discover(store.clone(), origin, issuer, "iris-local".into(), None).await?;
    let app = app(auth).with_state(AppState {
        database: database.to_owned(),
        now: unix_time,
        connections,
    });
    let listener = tokio::net::TcpListener::bind(listen).await?;
    // Registered before readiness is announced, so a supervisor that signals
    // as soon as it sees the address gets a drain.
    let signal = lifecycle::signals()?;
    eprintln!("Iris reference application; {data}, local test identities only.");
    eprintln!("listening on http://{}", listener.local_addr()?);
    let tasks = vec![lifecycle::session_cleanup(store, lifecycle::CLEANUP_PERIOD)];
    Ok(lifecycle::serve(listener, app, tasks, signal, lifecycle::DRAIN).await)
}

/// Refuses a populated development database that cannot resolve a fresh login
/// from the configured issuer. An empty mapping table is valid and is not
/// seeded here.
async fn check_identity_issuer(
    pool: &sqlx::SqlitePool,
    database: &Path,
    issuer: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (any, matching): (bool, bool) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM iris_external_identities), \
         EXISTS(SELECT 1 FROM iris_external_identities WHERE issuer = ?)",
    )
    .bind(issuer)
    .fetch_one(pool)
    .await
    .map_err(|error| format!("could not inspect external identity issuer mappings: {error}"))?;
    if !any {
        eprintln!(
            "reference-dev: warning: database {} has no external identity mappings; startup \
             does not seed existing databases",
            database.display()
        );
    } else if !matching {
        return Err(format!(
            "database {} has no matching IRIS_OIDC_ISSUER identity mappings, so a fresh login \
             cannot resolve; preserve the database by restoring the intended matching issuer; \
             to intentionally discard it instead, run: {}",
            database.display(),
            reset_command(database)
        )
        .into());
    }
    Ok(())
}
