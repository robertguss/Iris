//! Process lifecycle after startup (S18): supervised periodic tasks, one
//! shutdown timeline on SIGINT or SIGTERM, and acknowledged connection
//! closure before the database's ownership lock is released.
use std::{
    borrow::{Borrow, BorrowMut},
    future::Future,
    io,
    ops::{Deref, DerefMut},
    path::Path,
    pin::Pin,
    sync::Arc,
    task::Poll,
    time::Duration,
};

use axum::Router;
use sqlx::{Connection, SqliteConnection, SqlitePool};
use tokio::sync::watch;

use crate::{identity::store::Store, storage::Storage};

/// How long requests and tasks may take to finish after a stop begins.
pub const DRAIN: Duration = Duration::from_secs(3);
/// How long the session pool and the domain connections may take to close
/// after a complete drain.
pub const CLOSE: Duration = Duration::from_secs(1);
/// How often expired sessions and login attempts are deleted.
pub const CLEANUP_PERIOD: Duration = Duration::from_secs(60);

fn report(line: impl std::fmt::Display) {
    eprintln!("reference-dev: {line}");
}

/// Counts the domain connections whose closure has not been acknowledged.
/// One per process, shared by every clone of its `AppState`.
#[derive(Clone)]
pub struct Connections(Arc<watch::Sender<usize>>);

impl Default for Connections {
    fn default() -> Self {
        Self(Arc::new(watch::Sender::new(0)))
    }
}

impl Connections {
    /// Opens a connection that counts until its awaited close succeeds. The
    /// ticket is taken before the opening is awaited, and a failed or
    /// cancelled opening keeps it: SQLx can create a connection before
    /// `connect` fails, and no handle is left to acknowledge its closure.
    pub async fn open(&self, path: &Path) -> Result<Tracked, sqlx::Error> {
        self.0.send_modify(|outstanding| *outstanding += 1);
        let conn = crate::app::connect(path).await?;
        Ok(Tracked {
            conn: Some(conn),
            connections: self.clone(),
        })
    }

    /// How many opened connections have no acknowledged closure.
    pub fn outstanding(&self) -> usize {
        *watch::Sender::borrow(&self.0)
    }

    /// Resolves once no ticket is outstanding. It checks the current count
    /// first, so an acknowledgment before the call is not missed.
    pub async fn closed(&self) {
        let mut count = self.0.subscribe();
        // The sender lives in `self`, so the channel cannot close.
        let _ = count.wait_for(|outstanding| *outstanding == 0).await;
    }
}

/// A domain connection that counts as open until its closure is
/// acknowledged. Dropping it queues an awaited close on the current runtime;
/// only that close succeeding releases the ticket. Without a runtime, or if
/// the queued close never runs or fails, the ticket stays outstanding.
pub struct Tracked {
    conn: Option<SqliteConnection>,
    connections: Connections,
}

impl Tracked {
    fn conn(&self) -> &SqliteConnection {
        self.conn.as_ref().expect("taken only when dropped")
    }

    fn conn_mut(&mut self) -> &mut SqliteConnection {
        self.conn.as_mut().expect("taken only when dropped")
    }
}

impl Deref for Tracked {
    type Target = SqliteConnection;
    fn deref(&self) -> &SqliteConnection {
        self.conn()
    }
}

impl DerefMut for Tracked {
    fn deref_mut(&mut self) -> &mut SqliteConnection {
        self.conn_mut()
    }
}

impl Borrow<SqliteConnection> for Tracked {
    fn borrow(&self) -> &SqliteConnection {
        self.conn()
    }
}

impl BorrowMut<SqliteConnection> for Tracked {
    fn borrow_mut(&mut self) -> &mut SqliteConnection {
        self.conn_mut()
    }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        let Some(conn) = self.conn.take() else {
            return;
        };
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let connections = self.connections.clone();
        runtime.spawn(async move {
            if conn.close().await.is_ok() {
                connections.0.send_modify(|outstanding| *outstanding -= 1);
            }
        });
    }
}

/// Tells a task that a stop has begun.
#[derive(Clone)]
pub struct Shutdown(watch::Receiver<bool>);

impl Shutdown {
    /// Resolves once a stop has been requested, however long ago.
    pub async fn requested(&mut self) {
        // A dropped sender means the server is gone: also a stop.
        let _ = self.0.wait_for(|requested| *requested).await;
    }

    pub fn is_requested(&self) -> bool {
        *self.0.borrow()
    }
}

fn channel() -> (watch::Sender<bool>, Shutdown) {
    let (sender, receiver) = watch::channel(false);
    (sender, Shutdown(receiver))
}

type Start = Box<dyn FnOnce(Shutdown) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

/// Work the process owns for its whole life. It returns once a stop is
/// requested; returning earlier, or panicking, stops the process.
pub struct Task {
    name: &'static str,
    start: Start,
}

impl Task {
    pub fn new<F, Fut>(name: &'static str, start: F) -> Self
    where
        F: FnOnce(Shutdown) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self {
            name,
            start: Box::new(move |shutdown| Box::pin(start(shutdown))),
        }
    }
}

/// Runs `unit` at once and then every `period` until a stop is requested. A
/// stop wins over a tick that is ready at the same time, and a unit that has
/// started runs to its end.
pub fn periodic<F, Fut>(name: &'static str, period: Duration, mut unit: F) -> Task
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    Task::new(name, move |mut shutdown| async move {
        let mut ticks = tokio::time::interval(period);
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                biased;
                () = shutdown.requested() => return,
                _ = ticks.tick() => {}
            }
            unit().await;
        }
    })
}

/// Deletes expired sessions and login attempts. A database error is reported
/// and the next tick tries again; only the task ending stops the process.
pub fn session_cleanup(store: Store, period: Duration) -> Task {
    periodic("session-cleanup", period, move || {
        let store = store.clone();
        async move {
            if let Err(error) = store.cleanup().await {
                report(format_args!(
                    "session cleanup failed and will run again at the next tick: {error}"
                ));
            }
        }
    })
}

/// Something that went wrong while serving or draining.
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    /// A task panicked, at any time, or returned before a stop was requested.
    Task { name: &'static str, panicked: bool },
    /// The server ended before a stop was requested, or with an error.
    Server(String),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Task {
                name,
                panicked: true,
            } => write!(f, "task {name} panicked"),
            Self::Task { name, .. } => write!(f, "task {name} stopped unexpectedly"),
            Self::Server(why) => write!(f, "the server {why}"),
        }
    }
}

/// How serving ended. The cause, the drain and the failures are kept apart:
/// a panic can precede an expired drain, and a signal can precede a panic.
#[derive(Debug)]
pub struct Stopped {
    /// The signal that requested the stop, if a signal did.
    pub signal: Option<&'static str>,
    /// Whether the server and every task finished before the drain deadline.
    pub drained: bool,
    pub failures: Vec<Failure>,
}

impl Stopped {
    /// Only a stop requested by a signal, fully drained, without a failure
    /// and with every connection's closure acknowledged is a clean one.
    pub fn clean(&self, closed: bool) -> bool {
        self.signal.is_some() && self.drained && self.failures.is_empty() && closed
    }

    /// The closing diagnostics. Neither a clean stop nor an expired deadline
    /// is a receipt or a rollback acknowledgment for a request in flight.
    pub fn lines(&self, closed: bool, close: Duration) -> Vec<String> {
        if !self.drained {
            return vec![
                "drain deadline expired with requests or tasks still running; their outcomes \
                 are unconfirmed"
                    .into(),
                "terminating with the database lock held; connection closure was not established"
                    .into(),
            ];
        }
        if !closed {
            return vec![format!(
                "connections did not close within {close:?}; terminating with the database lock \
                 held"
            )];
        }
        if self.clean(closed) {
            vec!["stopped after draining".into()]
        } else {
            vec!["stopped after a failure".into()]
        }
    }
}

/// Registers the SIGINT and SIGTERM handlers now, inside the runtime, and
/// returns a future naming the first one received. Call it before announcing
/// readiness: until then a signal ends the process outright.
pub fn signals() -> io::Result<impl Future<Output = &'static str>> {
    use tokio::signal::unix::{SignalKind, signal};
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;
    Ok(async move {
        tokio::select! {
            _ = interrupt.recv() => "SIGINT",
            _ = terminate.recv() => "SIGTERM",
        }
    })
}

/// Whether shutdown had been requested when the task returned, read in the
/// task itself so that a later look at the handle cannot reclassify it.
type Handles = Vec<(&'static str, tokio::task::JoinHandle<bool>)>;

/// The next task to end; never resolves when none is left.
async fn ended(handles: &mut Handles) -> Option<Failure> {
    std::future::poll_fn(|context| {
        for index in 0..handles.len() {
            if let Poll::Ready(result) = Pin::new(&mut handles[index].1).poll(context) {
                let (name, _) = handles.swap_remove(index);
                return Poll::Ready(match result {
                    Ok(true) => None,
                    Ok(false) => Some(Failure::Task {
                        name,
                        panicked: false,
                    }),
                    Err(_) => Some(Failure::Task {
                        name,
                        panicked: true,
                    }),
                });
            }
        }
        Poll::Pending
    })
    .await
}

/// Serves until a signal, a task ending or the server ending, then follows
/// one timeline: the stop is published, so the server stops accepting and
/// tasks start no new work; requests and work in flight may finish until
/// `drain` has passed; and the result says what happened. The deadline starts
/// at the first cause, and a later signal does not move it.
pub async fn serve(
    listener: tokio::net::TcpListener,
    app: Router,
    tasks: Vec<Task>,
    signal: impl Future<Output = &'static str>,
    drain: Duration,
) -> Stopped {
    let (stop, shutdown) = channel();
    let mut handles: Handles = tasks
        .into_iter()
        .map(|task| {
            let shutdown = shutdown.clone();
            // The factory runs inside the watched task, so its panic is the
            // task's, like a panic in the future it returns.
            let handle = tokio::spawn(async move {
                (task.start)(shutdown.clone()).await;
                shutdown.is_requested()
            });
            (task.name, handle)
        })
        .collect();
    let mut server = tokio::spawn(async move {
        let mut requested = shutdown.clone();
        let served = axum::serve(listener, app)
            .with_graceful_shutdown(async move { requested.requested().await })
            .await;
        (served, shutdown.is_requested())
    });
    let mut stopped = Stopped {
        signal: None,
        drained: false,
        failures: Vec::new(),
    };
    let mut serving = true;
    tokio::pin!(signal);
    tokio::select! {
        biased;
        name = &mut signal => {
            report(format_args!("received {name}; draining for up to {drain:?}"));
            stopped.signal = Some(name);
        }
        failure = ended(&mut handles), if !handles.is_empty() => {
            stopped.failures.extend(failure);
        }
        result = &mut server => {
            serving = false;
            stopped.failures.extend(server_failure(result));
        }
    }
    for failure in &stopped.failures {
        report(format_args!("{failure}; stopping"));
    }
    let _ = stop.send(true);
    let deadline = tokio::time::Instant::now() + drain;
    stopped.drained = tokio::time::timeout_at(deadline, async {
        while serving || !handles.is_empty() {
            let failure = tokio::select! {
                failure = ended(&mut handles), if !handles.is_empty() => failure,
                result = &mut server, if serving => {
                    serving = false;
                    server_failure(result)
                }
            };
            if let Some(failure) = failure {
                report(&failure);
                stopped.failures.push(failure);
            }
        }
    })
    .await
    .is_ok();
    stopped
}

type Served = Result<(io::Result<()>, bool), tokio::task::JoinError>;

fn server_failure(result: Served) -> Option<Failure> {
    match result {
        Ok((Ok(()), true)) => None,
        Ok((Ok(()), false)) => Some(Failure::Server("stopped unexpectedly".into())),
        Ok((Err(error), _)) => Some(Failure::Server(format!("failed: {error}"))),
        Err(_) => Some(Failure::Server("panicked".into())),
    }
}

/// Closes the session pool and waits for every domain connection's
/// acknowledged closure, both under one deadline. False means closure was
/// not established; nothing is acknowledged by the timeout.
pub async fn closure(pool: &SqlitePool, connections: &Connections, close: Duration) -> bool {
    tokio::time::timeout(close, async {
        pool.close().await;
        connections.closed().await;
    })
    .await
    .is_ok()
}

/// What `finish` ends: the runtime, the owned database and its connections.
pub struct Process {
    pub runtime: tokio::runtime::Runtime,
    pub storage: Storage,
    pub pool: SqlitePool,
    pub connections: Connections,
}

/// Ends the process after `serve`, with its exit code. After a complete
/// drain, connections get `close` to close while the runtime still
/// runs. If their closure is acknowledged, the ownership lock is released
/// (and a disposable database removed) in the ordinary way. If it is not, or
/// the drain expired, the process terminates with the lock still held, so no
/// other owner can start while a connection of this one may be live; a
/// disposable directory is then left behind.
pub fn finish(process: Process, stopped: Stopped, close: Duration) -> ! {
    let Process {
        runtime,
        storage,
        pool,
        connections,
    } = process;
    let closed = stopped.drained && runtime.block_on(closure(&pool, &connections, close));
    for line in stopped.lines(closed, close) {
        report(line);
    }
    if !closed {
        before_exit();
        // No destructor runs: `storage` and its lock last until the
        // operating system ends the process and every connection with it.
        std::process::exit(1);
    }
    // Closure is established by the acknowledgments above, not by this.
    runtime.shutdown_background();
    drop(storage);
    std::process::exit(if stopped.clean(closed) { 0 } else { 1 })
}

#[cfg(not(test))]
fn before_exit() {}

#[cfg(test)]
pub(crate) const BARRIER: &str = "IRIS_LIFECYCLE_BARRIER";

/// Test-only: a forced termination waits here, with everything still held,
/// until the parent test writes a line.
#[cfg(test)]
fn before_exit() {
    if std::env::var_os(BARRIER).is_some() {
        println!("barrier before-exit");
        let _ = std::io::stdin().read_line(&mut String::new());
    }
}

#[cfg(test)]
pub(crate) struct Gate {
    reached: tokio::sync::Semaphore,
    release: tokio::sync::Semaphore,
}

#[cfg(test)]
impl Gate {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            reached: tokio::sync::Semaphore::new(0),
            release: tokio::sync::Semaphore::new(0),
        })
    }

    /// Called at the gated point: reports arrival, then waits for release.
    pub(crate) async fn pass(&self) {
        self.reached.add_permits(1);
        self.release.acquire().await.unwrap().forget();
    }

    pub(crate) async fn reached(&self) {
        self.reached.acquire().await.unwrap().forget();
    }

    pub(crate) fn release(&self) {
        self.release.add_permits(1);
    }
}

#[cfg(test)]
mod tests;
