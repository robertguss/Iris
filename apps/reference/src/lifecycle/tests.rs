// Tests that spawn a child hold `storage::exclusive()` for their whole
// length, on a runtime whose test future stays on one thread.
#![allow(clippy::await_holding_lock)]

use super::*;
use crate::{
    app::{AppState, app, connect, unix_time},
    domains::memberships::gate,
    http::memberships::tests::{Fixture, ORIGIN, body, provider},
    identity::Auth,
    storage::{StorageError, exclusive, shared},
};
use axum::routing::get;
use serde_json::{Value, json};
use sqlx::Connection;
use std::{
    io::{BufRead, BufReader, Write},
    net::SocketAddr,
    path::PathBuf,
    process::{ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering::SeqCst},
        mpsc,
    },
    time::Instant,
};
use tokio::{sync::oneshot, task::JoinHandle, time::timeout};
use tower_sessions::{SessionStore, session::Record};

const ISSUER: &str = "http://127.0.0.1:4001";
/// A drain deadline no passing test reaches.
const LONG: Duration = Duration::from_secs(20);
const SHORT: Duration = Duration::from_millis(400);
/// The bound on anything a test waits for.
const WAIT: Duration = Duration::from_secs(20);

fn canonical_dir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = std::fs::canonicalize(dir.path()).unwrap();
    (dir, path)
}

async fn pool(path: &Path, busy: Duration) -> SqlitePool {
    sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(path)
                .foreign_keys(true)
                .busy_timeout(busy),
        )
        .await
        .unwrap()
}

struct Running {
    address: SocketAddr,
    signal: oneshot::Sender<&'static str>,
    stopped: JoinHandle<Stopped>,
}

/// Serves `app` on a free port; the stop is requested through `signal`, and
/// dropping the sender requests nothing.
async fn start(app: Router, tasks: Vec<Task>, drain: Duration) -> Running {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (signal, received) = oneshot::channel();
    let signal_future = async move {
        match received.await {
            Ok(name) => name,
            Err(_) => std::future::pending().await,
        }
    };
    let stopped = tokio::spawn(serve(listener, app, tasks, signal_future, drain));
    Running {
        address,
        signal,
        stopped,
    }
}

async fn stopped(handle: JoinHandle<Stopped>) -> Stopped {
    timeout(WAIT, handle)
        .await
        .expect("the server did not stop")
        .unwrap()
}

/// Waits, bounded, until a new connection to `address` fails.
async fn refused(address: SocketAddr) {
    let started = Instant::now();
    while tokio::net::TcpStream::connect(address).await.is_ok() {
        assert!(started.elapsed() < WAIT, "still accepting connections");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// A router whose `/held` request waits at `gate`.
fn gated(gate: &Arc<Gate>) -> Router {
    let gate = gate.clone();
    Router::new().route(
        "/held",
        get(move || {
            let gate = gate.clone();
            async move {
                gate.pass().await;
                "done"
            }
        }),
    )
}

fn held(address: SocketAddr) -> JoinHandle<reqwest::Result<reqwest::Response>> {
    tokio::spawn(reqwest::get(format!("http://{address}/held")))
}

#[tokio::test]
async fn a_request_in_flight_at_the_signal_completes_and_the_stop_is_drained() {
    let gate = Gate::new();
    let running = start(gated(&gate), vec![], LONG).await;
    let request = held(running.address);
    timeout(WAIT, gate.reached()).await.unwrap();

    running.signal.send("SIGTERM").unwrap();
    refused(running.address).await;
    assert!(!running.stopped.is_finished());

    gate.release();
    let response = timeout(WAIT, request).await.unwrap().unwrap().unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "done");
    let stopped = stopped(running.stopped).await;
    assert_eq!(stopped.signal, Some("SIGTERM"));
    assert!(stopped.drained);
    assert_eq!(stopped.failures, []);
    assert!(stopped.clean(true));
    assert!(!stopped.clean(false));
}

#[tokio::test]
async fn a_request_never_finished_expires_the_drain_at_the_deadline() {
    let gate = Gate::new();
    let running = start(gated(&gate), vec![], SHORT).await;
    let request = held(running.address);
    timeout(WAIT, gate.reached()).await.unwrap();

    let signalled = Instant::now();
    running.signal.send("SIGINT").unwrap();
    let stopped = stopped(running.stopped).await;
    let elapsed = signalled.elapsed();
    assert!(elapsed >= SHORT, "stopped early, after {elapsed:?}");
    assert!(elapsed < SHORT + Duration::from_secs(2), "{elapsed:?}");
    assert_eq!(stopped.signal, Some("SIGINT"));
    assert!(!stopped.drained);
    assert_eq!(stopped.failures, []);
    assert!(!stopped.clean(true));
    // Never answered.
    assert!(!request.is_finished());
}

#[tokio::test]
async fn without_a_stop_the_server_keeps_serving() {
    let running = start(
        Router::new().route("/", get(|| async { "ok" })),
        vec![Task::new("waits", |mut shutdown| async move {
            shutdown.requested().await
        })],
        LONG,
    )
    .await;
    let url = format!("http://{}/", running.address);
    assert_eq!(reqwest::get(&url).await.unwrap().status(), 200);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!running.stopped.is_finished());
    running.signal.send("SIGTERM").unwrap();
    // The task returned only after the stop was requested: a normal return.
    let stopped = stopped(running.stopped).await;
    assert!(stopped.clean(true), "{stopped:?}");
}

/// A periodic task counting the units it starts and finishes; while `hold`
/// is set, a unit waits at `gate`.
struct Counter {
    started: Arc<AtomicUsize>,
    finished: Arc<AtomicUsize>,
    hold: Arc<AtomicBool>,
    gate: Arc<Gate>,
}

impl Counter {
    fn new() -> Self {
        Self {
            started: Default::default(),
            finished: Default::default(),
            hold: Default::default(),
            gate: Gate::new(),
        }
    }

    fn task(&self, period: Duration) -> Task {
        let (started, finished) = (self.started.clone(), self.finished.clone());
        let (hold, gate) = (self.hold.clone(), self.gate.clone());
        periodic("counter", period, move || {
            let (started, finished) = (started.clone(), finished.clone());
            let (hold, gate) = (hold.clone(), gate.clone());
            async move {
                started.fetch_add(1, SeqCst);
                if hold.load(SeqCst) {
                    gate.pass().await;
                }
                finished.fetch_add(1, SeqCst);
            }
        })
    }
}

#[tokio::test]
async fn no_unit_starts_after_the_signal_and_one_in_flight_finishes() {
    let counter = Counter::new();
    let running = start(
        Router::new(),
        vec![counter.task(Duration::from_millis(10))],
        LONG,
    )
    .await;
    // The task ticks, first immediately and then repeatedly.
    let started = Instant::now();
    while counter.finished.load(SeqCst) < 3 {
        assert!(started.elapsed() < WAIT, "the task never ticked");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    counter.hold.store(true, SeqCst);
    timeout(WAIT, counter.gate.reached()).await.unwrap();
    let units = counter.started.load(SeqCst);

    running.signal.send("SIGTERM").unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    // The unit in flight is awaited, not abandoned.
    assert!(!running.stopped.is_finished());
    assert_eq!(counter.finished.load(SeqCst), units - 1);

    counter.gate.release();
    let stopped = stopped(running.stopped).await;
    assert!(stopped.clean(true), "{stopped:?}");
    assert_eq!(counter.started.load(SeqCst), units);
    assert_eq!(counter.finished.load(SeqCst), units);
}

#[tokio::test]
async fn a_stop_is_seen_by_a_late_subscriber_and_wins_over_a_ready_tick() {
    let (sender, _first) = channel();
    sender.send(true).unwrap();
    // A subscriber created after the request still sees it.
    let mut late = Shutdown(sender.subscribe());
    timeout(WAIT, late.requested()).await.unwrap();
    assert!(late.is_requested());

    // Parked at its select with nothing ready; whether the first tick ran a
    // unit on the way there does not matter.
    let (sender, shutdown) = channel();
    let counter = Counter::new();
    let mut task = (counter.task(Duration::from_millis(50)).start)(shutdown);
    let parked = std::future::poll_fn(|context| {
        std::task::Poll::Ready(task.as_mut().poll(context).is_pending())
    });
    assert!(parked.await);
    let units = counter.started.load(SeqCst);
    // The task is not polled while the next tick comes due and the stop is
    // published, so its select is reached with both ready: no unit starts.
    tokio::time::sleep(Duration::from_millis(150)).await;
    sender.send(true).unwrap();
    timeout(WAIT, task).await.unwrap();
    assert_eq!(counter.started.load(SeqCst), units);
}

#[tokio::test]
async fn a_task_started_after_the_stop_runs_no_unit() {
    let (sender, shutdown) = channel();
    sender.send(true).unwrap();
    let counter = Counter::new();
    let task = counter.task(Duration::from_millis(10));
    timeout(WAIT, (task.start)(shutdown)).await.unwrap();
    assert_eq!(counter.started.load(SeqCst), 0);
}

#[tokio::test]
async fn a_task_that_panics_or_returns_early_stops_the_server_and_is_a_failure() {
    for (panics, expected) in [(true, true), (false, false)] {
        let task = Task::new("doomed", move |_| async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            assert!(!panics, "doomed task");
        });
        let running = start(Router::new(), vec![task], LONG).await;
        let address = running.address;
        let stopped = stopped(running.stopped).await;
        assert_eq!(stopped.signal, None);
        assert!(stopped.drained);
        assert_eq!(
            stopped.failures,
            [Failure::Task {
                name: "doomed",
                panicked: expected
            }]
        );
        assert!(!stopped.clean(true));
        refused(address).await;
    }
}

#[tokio::test]
async fn a_panic_in_a_task_factory_is_that_tasks_failure_and_the_rest_drains() {
    let counter = Counter::new();
    let broken = Task::new("factory", |_| -> std::future::Pending<()> {
        panic!("factory panicked before returning its future")
    });
    let tasks = vec![counter.task(Duration::from_millis(10)), broken];
    let running = start(Router::new(), tasks, LONG).await;
    let address = running.address;
    let stopped = stopped(running.stopped).await;
    assert_eq!(stopped.signal, None);
    // The other task was told to stop and returned.
    assert!(stopped.drained);
    assert_eq!(
        stopped.failures,
        [Failure::Task {
            name: "factory",
            panicked: true
        }]
    );
    refused(address).await;
    let units = counter.started.load(SeqCst);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.started.load(SeqCst), units);
}

#[tokio::test]
async fn a_task_that_ignores_the_stop_expires_the_drain() {
    let task = Task::new("deaf", |_| std::future::pending());
    let running = start(Router::new(), vec![task], SHORT).await;
    let signalled = Instant::now();
    running.signal.send("SIGTERM").unwrap();
    let stopped = stopped(running.stopped).await;
    assert!(signalled.elapsed() >= SHORT);
    assert!(!stopped.drained);
    assert_eq!(stopped.failures, []);
}

#[tokio::test]
async fn a_panic_while_draining_is_kept_and_the_stop_is_not_clean() {
    let task = Task::new("late", |mut shutdown| async move {
        shutdown.requested().await;
        panic!("late panic");
    });
    let running = start(Router::new(), vec![task], LONG).await;
    running.signal.send("SIGTERM").unwrap();
    let stopped = stopped(running.stopped).await;
    assert_eq!(stopped.signal, Some("SIGTERM"));
    assert!(stopped.drained);
    assert_eq!(
        stopped.failures,
        [Failure::Task {
            name: "late",
            panicked: true
        }]
    );
    assert!(!stopped.clean(true));
}

#[tokio::test]
async fn a_panic_followed_by_an_expired_drain_keeps_both() {
    let gate = Gate::new();
    let trigger = Gate::new();
    let waits = trigger.clone();
    let task = Task::new("doomed", move |_| async move {
        waits.pass().await;
        panic!("doomed task");
    });
    let running = start(gated(&gate), vec![task], SHORT).await;
    let request = held(running.address);
    timeout(WAIT, gate.reached()).await.unwrap();

    trigger.release();
    let stopped = stopped(running.stopped).await;
    assert_eq!(stopped.signal, None);
    assert!(!stopped.drained);
    assert_eq!(
        stopped.failures,
        [Failure::Task {
            name: "doomed",
            panicked: true
        }]
    );
    assert!(!request.is_finished());
    let lines = stopped.lines(false, CLOSE);
    assert!(lines.iter().any(|line| line.contains("unconfirmed")));
}

#[tokio::test]
async fn an_early_return_that_races_the_signal_is_still_an_early_return() {
    // The signal becomes ready in the same poll in which the task returns,
    // before the stop is published.
    let (done, returned) = oneshot::channel();
    let task = Task::new("early", move |_| async move {
        done.send(()).unwrap();
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let signal = async move {
        returned.await.unwrap();
        "SIGTERM"
    };
    let stopped = timeout(
        WAIT,
        serve(listener, Router::new(), vec![task], signal, LONG),
    )
    .await
    .unwrap();
    assert!(stopped.drained);
    assert_eq!(
        stopped.failures,
        [Failure::Task {
            name: "early",
            panicked: false
        }]
    );
    assert!(!stopped.clean(true));
}

#[test]
fn the_messages_name_each_outcome_and_only_a_signalled_drained_closed_stop_is_clean() {
    let stop = |signal, drained, failures| Stopped {
        signal,
        drained,
        failures,
    };
    let clean = stop(Some("SIGTERM"), true, vec![]);
    assert!(clean.clean(true));
    assert_eq!(clean.lines(true, CLOSE), ["stopped after draining"]);

    // A drained stop whose close phase ran out of time.
    assert!(!clean.clean(false));
    let lines = clean.lines(false, CLOSE).join("\n");
    assert!(lines.contains("did not close within 1s"), "{lines}");
    assert!(lines.contains("database lock held"), "{lines}");
    assert!(!lines.contains("unconfirmed"), "{lines}");

    // An expired drain: closure was skipped, not timed.
    let expired = stop(Some("SIGTERM"), false, vec![]);
    assert!(!expired.clean(false));
    let lines = expired.lines(false, CLOSE).join("\n");
    assert!(lines.contains("unconfirmed"), "{lines}");
    assert!(lines.contains("closure was not established"), "{lines}");
    assert!(!lines.contains("did not close within"), "{lines}");

    let failed = stop(
        None,
        true,
        vec![Failure::Task {
            name: "session-cleanup",
            panicked: true,
        }],
    );
    assert!(!failed.clean(true));
    assert_eq!(failed.lines(true, CLOSE), ["stopped after a failure"]);
    // Only a signal makes a stop clean, whatever else is in order.
    assert!(!stop(None, true, vec![]).clean(true));
}

async fn identifiers(pool: &SqlitePool) -> Vec<String> {
    let mut ids: Vec<String> = sqlx::query_scalar("SELECT id FROM iris_sessions")
        .fetch_all(pool)
        .await
        .unwrap();
    ids.extend(
        sqlx::query_scalar::<_, String>("SELECT state FROM iris_login_attempts")
            .fetch_all(pool)
            .await
            .unwrap(),
    );
    ids.sort();
    ids
}

async fn plant(pool: &SqlitePool, name: &str, expires_at: i64) {
    sqlx::query("INSERT INTO iris_sessions(id,data,expires_at) VALUES(?,'{}',?)")
        .bind(format!("session-{name}"))
        .bind(expires_at)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO iris_login_attempts(state,browser_id,nonce,verifier,expires_at) VALUES(?,'b','n','v',?)",
    )
    .bind(format!("attempt-{name}"))
    .bind(expires_at)
    .execute(pool)
    .await
    .unwrap();
}

async fn eventually(pool: &SqlitePool, expected: &[&str]) {
    let started = Instant::now();
    loop {
        let ids = identifiers(pool).await;
        if ids == expected {
            return;
        }
        assert!(started.elapsed() < WAIT, "still {ids:?}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

static CLEANUP_CLOCK: AtomicI64 = AtomicI64::new(1_000);

#[tokio::test]
async fn session_cleanup_deletes_expired_rows_on_a_persistent_database_and_outlives_a_busy_tick() {
    let _processes = shared();
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    let storage = Storage::open(&path, ISSUER).await.unwrap();
    let pool = pool(&path, Duration::from_millis(30)).await;
    let store = Store {
        pool: pool.clone(),
        now: || CLEANUP_CLOCK.load(SeqCst),
    };
    plant(&pool, "old", 1_000).await;
    plant(&pool, "soon", 2_000).await;
    plant(&pool, "live", 9_000).await;

    let task = session_cleanup(store, Duration::from_millis(30));
    let running = start(Router::new(), vec![task], LONG).await;
    // The first tick, at start.
    let kept = [
        "attempt-live",
        "attempt-soon",
        "session-live",
        "session-soon",
    ];
    eventually(&pool, &kept).await;

    // A writer holds the database past the pool's busy timeout, so ticks
    // fail; the task reports them and keeps running.
    let mut writer = connect(&path).await.unwrap();
    let tx = writer.begin_with("BEGIN IMMEDIATE").await.unwrap();
    CLEANUP_CLOCK.store(2_000, SeqCst);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(identifiers(&pool).await, kept);
    assert!(!running.stopped.is_finished());
    tx.rollback().await.unwrap();
    writer.close().await.unwrap();
    // A later tick.
    eventually(&pool, &["attempt-live", "session-live"]).await;

    running.signal.send("SIGTERM").unwrap();
    let stopped = stopped(running.stopped).await;
    assert!(stopped.clean(true), "{stopped:?}");
    assert!(closure(&pool, &Connections::default(), CLOSE).await);
    drop(storage);
}

#[tokio::test]
async fn a_cleanup_in_flight_at_the_signal_runs_to_its_end() {
    let _processes = shared();
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    let _storage = Storage::open(&path, ISSUER).await.unwrap();
    let pool = pool(&path, Duration::from_secs(10)).await;
    plant(&pool, "old", 1).await;
    let store = Store {
        pool: pool.clone(),
        now: unix_time,
    };
    // The first tick's first statement waits for this writer.
    let mut writer = connect(&path).await.unwrap();
    let tx = writer.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let running = start(
        Router::new(),
        vec![session_cleanup(store, CLEANUP_PERIOD)],
        LONG,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    running.signal.send("SIGTERM").unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!running.stopped.is_finished());
    tx.rollback().await.unwrap();
    let stopped = stopped(running.stopped).await;
    assert!(stopped.clean(true), "{stopped:?}");
    // Both statements ran: the unit was not cut short by the stop.
    assert_eq!(identifiers(&pool).await, [] as [&str; 0]);
}

#[tokio::test]
async fn a_ticket_is_acknowledged_only_by_an_awaited_close() {
    let (_dir, root) = canonical_dir();
    let path = root.join("plain.db");
    let connections = Connections::default();
    let mut conn = connections.open(&path).await.unwrap();
    // In WAL mode the `-wal` file lasts exactly as long as a connection is
    // open, which makes the close itself observable.
    sqlx::raw_sql("PRAGMA journal_mode=WAL; CREATE TABLE t (v)")
        .execute(&mut *conn)
        .await
        .unwrap();
    let wal = root.join("plain.db-wal");
    assert!(wal.exists());
    assert_eq!(connections.outstanding(), 1);
    assert!(
        timeout(Duration::from_millis(100), connections.closed())
            .await
            .is_err()
    );
    // Dropping queues the close; nothing is acknowledged by the drop itself.
    drop(conn);
    assert_eq!(connections.outstanding(), 1);
    timeout(WAIT, connections.closed()).await.unwrap();
    assert_eq!(connections.outstanding(), 0);
    assert!(!wal.exists());

    // A failed open keeps its ticket: no handle exists to acknowledge it.
    assert!(
        connections
            .open(&root.join("missing/plain.db"))
            .await
            .is_err()
    );
    assert_eq!(connections.outstanding(), 1);
    assert!(
        timeout(Duration::from_millis(200), connections.closed())
            .await
            .is_err()
    );
    let pool = pool(&path, Duration::from_secs(1)).await;
    assert!(!closure(&pool, &connections, Duration::from_millis(200)).await);
}

/// SQLx runs a connection on its own thread. A ticket must outlast that
/// thread's work, not just the request to close.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_ticket_stays_outstanding_while_the_connections_worker_is_still_running() {
    let (_dir, root) = canonical_dir();
    let connections = Connections::default();
    let mut conn = connections.open(&root.join("plain.db")).await.unwrap();
    sqlx::raw_sql("PRAGMA journal_mode=WAL; CREATE TABLE t (v); INSERT INTO t VALUES (0)")
        .execute(&mut *conn)
        .await
        .unwrap();
    // The update hook holds the worker thread inside an UPDATE.
    let (entered, inside) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    conn.lock_handle().await.unwrap().set_update_hook(move |_| {
        let _ = entered.send(());
        let _ = released.recv_timeout(WAIT);
    });
    let update = tokio::spawn(async move {
        let _ = sqlx::query("UPDATE t SET v=1").execute(&mut *conn).await;
    });
    inside.recv_timeout(WAIT).unwrap();
    // Cancelling the statement drops the connection, which queues its close.
    update.abort();
    assert!(update.await.unwrap_err().is_cancelled());
    let early = timeout(Duration::from_millis(150), connections.closed())
        .await
        .is_ok();
    let outstanding = connections.outstanding();
    // Released before asserting, so a failure leaves no held worker.
    release.send(()).unwrap();
    assert!(!early, "acknowledged while the worker was inside an update");
    assert_eq!(outstanding, 1);
    timeout(WAIT, connections.closed()).await.unwrap();
    assert!(!root.join("plain.db-wal").exists());
}

#[test]
fn a_tracked_connection_dropped_outside_a_runtime_is_never_acknowledged() {
    let (_dir, root) = canonical_dir();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let connections = Connections::default();
    let conn = runtime
        .block_on(connections.open(&root.join("plain.db")))
        .unwrap();
    drop(conn);
    runtime.block_on(async { tokio::time::sleep(Duration::from_millis(200)).await });
    assert_eq!(connections.outstanding(), 1);
}

fn role_request(address: &str, cookie: &str) -> reqwest::RequestBuilder {
    reqwest::Client::new()
        .post(format!("{address}/api/memberships/role"))
        .header("content-type", "application/json")
        .header("cookie", cookie)
        .header("origin", ORIGIN)
        .header("x-iris-csrf", "csrf-canary")
        .body(body(29, "viewer"))
}

async fn role(path: &Path) -> String {
    let mut conn = connect(path).await.unwrap();
    let role =
        sqlx::query_scalar("SELECT role FROM memberships WHERE project_id=41 AND user_id=29")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    conn.close().await.unwrap();
    role
}

#[tokio::test]
async fn a_mutation_past_its_write_at_the_signal_commits_answers_and_closes() {
    let f = Fixture::new().await;
    let alice = f.cookie(Some(11)).await;
    let gate = gate::before_commit(&f.state.database);
    let running = start(f.app(), vec![], LONG).await;
    let address = format!("http://{}", running.address);
    // A read, so both kinds of tracked connection have been used.
    let members = reqwest::Client::new()
        .get(format!("{address}/api/projects/41/members"))
        .header("cookie", &alice)
        .send()
        .await
        .unwrap();
    assert_eq!(members.status(), 200);
    let request = tokio::spawn(role_request(&address, &alice).send());
    timeout(WAIT, gate.reached()).await.unwrap();
    // The write ran in a transaction that has not committed.
    assert_eq!(role(&f.state.database).await, "editor");

    running.signal.send("SIGTERM").unwrap();
    refused(running.address).await;
    assert!(!running.stopped.is_finished());
    gate.release();

    let response = timeout(WAIT, request).await.unwrap().unwrap().unwrap();
    assert_eq!(response.status(), 200);
    let envelope: Value = response.json().await.unwrap();
    assert_eq!(envelope["kind"], "success");
    let stopped = stopped(running.stopped).await;
    assert!(stopped.clean(true), "{stopped:?}");
    // Closure is acknowledged: the pool closes and no ticket is outstanding.
    assert!(closure(&f.store.pool, &f.state.connections, CLOSE).await);
    assert_eq!(f.state.connections.outstanding(), 0);
    assert_eq!(role(&f.state.database).await, "viewer");
}

const CHILD: &str = "IRIS_LIFECYCLE_CHILD";
const CHILD_PATH: &str = "IRIS_LIFECYCLE_PATH";
const CHILD_ISSUER: &str = "IRIS_LIFECYCLE_ISSUER";
const CHILD_DRAIN: Duration = Duration::from_millis(600);
const CHILD_CLOSE: Duration = Duration::from_millis(600);

/// The child half of the process tests; a no-op in an ordinary test run. It
/// is the development binary's sequence after startup, with one fault.
#[test]
fn child() {
    let Ok(mode) = std::env::var(CHILD) else {
        return;
    };
    let path = PathBuf::from(std::env::var_os(CHILD_PATH).unwrap());
    let issuer = std::env::var(CHILD_ISSUER).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (storage, pool, connections, stopped) = runtime.block_on(async {
        let storage = Storage::open(&path, &issuer).await.unwrap();
        let pool = pool(storage.path(), Duration::from_secs(1)).await;
        let store = Store {
            pool: pool.clone(),
            now: unix_time,
        };
        let auth = Auth::discover(
            store.clone(),
            ORIGIN.into(),
            issuer,
            "iris-local".into(),
            None,
        )
        .await
        .unwrap();
        let connections = Connections::default();
        let state = AppState {
            database: storage.path().to_owned(),
            now: unix_time,
            connections: connections.clone(),
        };
        let mut tasks = Vec::new();
        match mode.as_str() {
            "transaction" => {
                let gate = gate::before_commit(storage.path());
                tokio::spawn(async move {
                    gate.reached().await;
                    println!("marker gate");
                });
            }
            "pool" => {
                let held = pool.acquire().await.unwrap();
                tokio::spawn(async move {
                    let _held = held;
                    std::future::pending::<()>().await
                });
            }
            "tracked" => {
                let held = connections.open(storage.path()).await.unwrap();
                tokio::spawn(async move {
                    let _held = held;
                    std::future::pending::<()>().await
                });
            }
            "panic" => tasks.push(Task::new("doomed", |_| async {
                tokio::time::sleep(Duration::from_millis(200)).await;
                panic!("doomed task")
            })),
            other => panic!("unknown mode {other}"),
        }
        let mut record = Record {
            id: Default::default(),
            data: [(
                "browser".into(),
                json!({"csrf": "csrf-canary", "user_id": 11, "expires_at": unix_time() + 3600}),
            )]
            .into(),
            expiry_date: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
        };
        store.create(&mut record).await.unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let signal = signals().unwrap();
        // libtest's own "test ... " prefix shares the first line.
        println!();
        println!("cookie=iris-session-dev={}", record.id);
        println!("listening=http://{}", listener.local_addr().unwrap());
        let stopped = serve(
            listener,
            app(auth).with_state(state),
            tasks,
            signal,
            CHILD_DRAIN,
        )
        .await;
        (storage, pool, connections, stopped)
    });
    finish(
        Process {
            runtime,
            storage,
            pool,
            connections,
        },
        stopped,
        CHILD_CLOSE,
    )
}

/// A child of this test binary, killed and reaped if a test fails.
struct Child {
    process: std::process::Child,
    stdin: Option<ChildStdin>,
    out: mpsc::Receiver<String>,
    err: mpsc::Receiver<String>,
}

impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

fn lines(stream: impl std::io::Read + Send + 'static) -> mpsc::Receiver<String> {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            if send.send(line).is_err() {
                break;
            }
        }
    });
    receive
}

impl Child {
    /// Runs `child` in `mode`; with `barrier`, a forced termination waits
    /// for `release` just before it exits.
    fn spawn(mode: &str, path: &Path, issuer: &str, barrier: bool) -> Self {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "lifecycle::tests::child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD, mode)
            .env(CHILD_PATH, path)
            .env(CHILD_ISSUER, issuer)
            .env_remove(BARRIER)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if barrier {
            command.env(BARRIER, "1");
        }
        let mut process = command.spawn().unwrap();
        Self {
            stdin: process.stdin.take(),
            out: lines(process.stdout.take().unwrap()),
            err: lines(process.stderr.take().unwrap()),
            process,
        }
    }

    /// Waits for a stdout line containing `marker` and returns what follows it.
    fn line(&self, marker: &str) -> String {
        loop {
            let line = self
                .out
                .recv_timeout(WAIT)
                .unwrap_or_else(|_| panic!("the child never printed {marker}"));
            if let Some((_, rest)) = line.split_once(marker) {
                return rest.to_owned();
            }
        }
    }

    fn signal(&self, name: &str) {
        let status = Command::new("kill")
            .args([format!("-{name}"), self.process.id().to_string()])
            .status()
            .unwrap();
        assert!(status.success());
    }

    /// Lets a child waiting before its exit proceed.
    fn release(&mut self) {
        let mut stdin = self.stdin.take().unwrap();
        writeln!(stdin).unwrap();
    }

    /// The child's own exit code and its whole stderr.
    fn exit(&mut self) -> (Option<i32>, String) {
        let started = Instant::now();
        let status = loop {
            if let Some(status) = self.process.try_wait().unwrap() {
                break status;
            }
            assert!(started.elapsed() < WAIT, "the child is still running");
            std::thread::sleep(Duration::from_millis(20));
        };
        (
            status.code(),
            self.err.iter().collect::<Vec<_>>().join("\n"),
        )
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_transaction_abandoned_at_the_drain_deadline_keeps_the_lock_until_the_process_ends() {
    let _processes = exclusive();
    let (_provider, issuer) = provider();
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    let mut child = Child::spawn("transaction", &path, &issuer, true);
    let cookie = child.line("cookie=");
    let address = child.line("listening=");
    let request = tokio::spawn(role_request(&address, &cookie).send());
    child.line("marker gate");

    let signalled = Instant::now();
    child.signal("TERM");
    child.line("barrier before-exit");
    let elapsed = signalled.elapsed();
    assert!(
        elapsed >= CHILD_DRAIN,
        "terminated early, after {elapsed:?}"
    );
    // No close phase follows an expired drain.
    assert!(elapsed < CHILD_DRAIN + CHILD_CLOSE, "{elapsed:?}");
    // The lock is held while the process still has the transaction open.
    assert!(matches!(
        Storage::open(&path, &issuer).await,
        Err(StorageError::InUse(_))
    ));
    assert!(!request.is_finished());

    child.release();
    let (code, stderr) = child.exit();
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("unconfirmed"), "{stderr}");
    assert!(stderr.contains("closure was not established"), "{stderr}");
    assert!(!stderr.contains("did not close within"), "{stderr}");
    // Never acknowledged.
    assert!(timeout(WAIT, request).await.unwrap().unwrap().is_err());

    // For this transaction on this run: reopening rolls it back.
    let storage = Storage::open(&path, &issuer).await.unwrap();
    assert_eq!(role(storage.path()).await, "editor");
}

#[tokio::test]
async fn a_connection_that_never_closes_ends_in_a_forced_exit_with_the_lock_held() {
    let _processes = exclusive();
    let (_provider, issuer) = provider();
    for mode in ["pool", "tracked"] {
        let (_dir, root) = canonical_dir();
        let path = root.join("dev.db");
        let mut child = Child::spawn(mode, &path, &issuer, true);
        child.line("listening=");

        let signalled = Instant::now();
        child.signal("TERM");
        child.line("barrier before-exit");
        let elapsed = signalled.elapsed();
        assert!(elapsed >= CHILD_CLOSE, "{mode}: early, after {elapsed:?}");
        assert!(
            elapsed < CHILD_CLOSE + Duration::from_secs(3),
            "{mode}: {elapsed:?}"
        );
        assert!(
            matches!(
                Storage::open(&path, &issuer).await,
                Err(StorageError::InUse(_))
            ),
            "{mode}"
        );

        child.release();
        let (code, stderr) = child.exit();
        assert_eq!(code, Some(1), "{mode}: {stderr}");
        assert!(stderr.contains("did not close within"), "{mode}: {stderr}");
        assert!(!stderr.contains("unconfirmed"), "{mode}: {stderr}");
        Storage::open(&path, &issuer).await.unwrap();
    }
}

#[tokio::test]
async fn a_panicking_task_stops_the_process_with_a_message() {
    let _processes = exclusive();
    let (_provider, issuer) = provider();
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    let mut child = Child::spawn("panic", &path, &issuer, false);
    child.line("listening=");
    let (code, stderr) = child.exit();
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("task doomed panicked"), "{stderr}");
    assert!(stderr.contains("stopped after a failure"), "{stderr}");
    // Drained and closed, so the lock was released in the ordinary way.
    Storage::open(&path, &issuer).await.unwrap();
}
