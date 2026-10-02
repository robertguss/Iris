//! Historical socket boundaries plus controlled actual-COMMIT future cancellation.
use super::tests::{Fixture, ORIGIN, body, request};
use crate::{app::connect, domains::memberships::gate, lifecycle::Gate};
use axum::{
    Router,
    extract::Request,
    middleware::{self, Next},
};
use sqlx::Connection;
use std::{
    future::Future,
    net::{Shutdown, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::oneshot,
    task::JoinHandle,
    time::timeout,
};
use tower::ServiceExt;

const WAIT: Duration = Duration::from_secs(10);

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    timeout(WAIT, future)
        .await
        .expect("bounded caller-loss wait expired")
}

async fn role(f: &Fixture) -> String {
    bounded(async {
        let mut conn = connect(&f.state.database).await.unwrap();
        let role =
            sqlx::query_scalar("SELECT role FROM memberships WHERE project_id=41 AND user_id=29")
                .fetch_one(&mut conn)
                .await
                .unwrap();
        conn.close().await.unwrap();
        role
    })
    .await
}

async fn closed_and_progress(f: &Fixture, expected: &str) {
    bounded(f.state.connections.closed()).await;
    assert_eq!(f.state.connections.outstanding(), 0);
    assert_eq!(role(f).await, expected, "final independently read role");
    bounded(async {
        let mut conn = connect(&f.state.database).await.unwrap();
        let mut tx = conn.begin_with("BEGIN IMMEDIATE").await.unwrap();
        sqlx::query("UPDATE memberships SET role='owner' WHERE project_id=41 AND user_id=29")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        conn.close().await.unwrap();
    })
    .await;
    assert_eq!(role(f).await, "owner", "independent writer made progress");
}

#[tokio::test]
async fn owned_future_abort_before_mutation() {
    let f = bounded(Fixture::new()).await;
    let cookie = bounded(f.cookie(Some(11))).await;
    let hold = gate::hold(&f.state.database, gate::Phase::BeforeMutation);
    let task = tokio::spawn(f.app().oneshot(request(&cookie, &body(29, "viewer"))));
    bounded(hold.gate.reached()).await;
    assert_eq!(f.state.connections.outstanding(), 1);
    task.abort();
    assert!(bounded(task).await.unwrap_err().is_cancelled());
    // Still held: closure cannot be explained by releasing the action.
    closed_and_progress(&f, "editor").await;
}

#[derive(Debug, PartialEq)]
enum End {
    Returned,
    Dropped,
    Panicked,
}

struct Observation(Option<oneshot::Sender<End>>);
impl Observation {
    fn returned(mut self) {
        let _ = self.0.take().unwrap().send(End::Returned);
    }
}
impl Drop for Observation {
    fn drop(&mut self) {
        if let Some(send) = self.0.take() {
            let _ = send.send(if std::thread::panicking() {
                End::Panicked
            } else {
                End::Dropped
            });
        }
    }
}

struct ResponseHold(Arc<Gate>);
impl Drop for ResponseHold {
    fn drop(&mut self) {
        self.0.release();
    }
}

struct Server {
    address: SocketAddr,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}
impl Server {
    async fn start(app: Router) -> Self {
        let listener = bounded(tokio::net::TcpListener::bind("127.0.0.1:0"))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        Self {
            address,
            stop: Some(stop),
            task: Some(task),
        }
    }
    async fn finish(mut self) {
        let _ = self.stop.take().unwrap().send(());
        bounded(self.task.take().unwrap()).await.unwrap();
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        // Axum's graceful shutdown also signals accepted connections. Do not
        // merely abort the accept loop and orphan their tasks on failure.
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

async fn socket(server: &Server, cookie: &str) -> TcpStream {
    let mut stream = bounded(TcpStream::connect(server.address)).await.unwrap();
    let body = body(29, "viewer");
    let framed = format!(
        "POST /api/memberships/role HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCookie: {cookie}\r\nOrigin: {ORIGIN}\r\nX-Iris-Csrf: csrf-canary\r\nConnection: close\r\n\r\n{body}",
        server.address,
        body.len()
    );
    bounded(stream.write_all(framed.as_bytes())).await.unwrap();
    stream
}

fn lose(stream: TcpStream) {
    let stream = stream.into_std().unwrap();
    stream.shutdown(Shutdown::Both).unwrap();
    drop(stream); // No cloned or retained caller handles.
}

#[derive(PartialEq)]
enum Loss {
    BeforeCommit,
    BeforeResponse,
    None,
}

async fn raw(loss: Loss) {
    let f = bounded(Fixture::new()).await;
    let cookie = bounded(f.cookie(Some(11))).await;
    let hold = gate::hold(&f.state.database, gate::Phase::BeforeCommit);
    let response_hold = ResponseHold(Gate::new());
    let response_gate = response_hold.0.clone();
    let (send, observed) = oneshot::channel();
    let send = Arc::new(std::sync::Mutex::new(Some(send)));
    let app = f
        .app()
        .layer(middleware::from_fn(move |request: Request, next: Next| {
            let observation = Observation(send.lock().unwrap().take());
            let gate = response_gate.clone();
            async move {
                let response = next.run(request).await;
                assert_eq!(response.status(), 200, "successful inner response");
                gate.pass().await;
                // The guard stays armed through the response hold. No await may
                // separate disarming/Returned from returning to Hyper.
                observation.returned();
                response
            }
        }));
    let server = Server::start(app).await;
    let mut stream = socket(&server, &cookie).await;
    bounded(hold.gate.reached()).await;
    assert_eq!(f.state.connections.outstanding(), 1);
    assert_eq!(role(&f).await, "editor", "uncommitted write is invisible");
    if loss == Loss::BeforeCommit {
        lose(stream);
        assert_eq!(
            bounded(observed).await.unwrap(),
            End::Dropped,
            "outer request must drop while pre-commit hold remains"
        );
        closed_and_progress(&f, "editor").await;
    } else {
        hold.gate.release();
        bounded(response_hold.0.reached()).await;
        assert_eq!(
            role(&f).await,
            "viewer",
            "commit precedes response exposure"
        );
        let mut byte = [0];
        assert!(
            timeout(Duration::from_millis(100), stream.read(&mut byte))
                .await
                .is_err(),
            "response exposed while held"
        );
        if loss == Loss::BeforeResponse {
            lose(stream);
            assert_eq!(
                bounded(observed).await.unwrap(),
                End::Dropped,
                "outer request must drop while response hold remains"
            );
        } else {
            response_hold.0.release();
            let mut bytes = Vec::new();
            bounded(stream.read_to_end(&mut bytes)).await.unwrap();
            let response = String::from_utf8(bytes).unwrap();
            let (headers, body) = response.split_once("\r\n\r\n").unwrap();
            assert!(headers.starts_with("HTTP/1.1 200 OK\r\n"), "{headers}");
            let length: usize = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .map(str::to_owned)
                })
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(body.len(), length, "complete response body");
            let value: serde_json::Value = serde_json::from_str(body).unwrap();
            assert_eq!(value["kind"], "success");
            assert_eq!(value["data"]["completion"], "acknowledged");
            let doc = serde_json::to_value(super::change_role().api).unwrap();
            let mut schema = doc["paths"]["/api/memberships/role"]["post"]["responses"]["200"]
                ["content"]["application/json"]["schema"].clone();
            schema["components"] = doc["components"].clone();
            assert!(
                jsonschema::draft202012::new(&schema)
                    .unwrap()
                    .is_valid(&value),
                "complete success envelope"
            );
            assert_eq!(bounded(observed).await.unwrap(), End::Returned);
        }
        closed_and_progress(&f, "viewer").await;
    }
    server.finish().await;
}

#[tokio::test]
async fn socket_loss_before_commit() {
    raw(Loss::BeforeCommit).await;
}

#[tokio::test]
async fn socket_loss_before_response_exposure() {
    raw(Loss::BeforeResponse).await;
}

#[tokio::test]
async fn same_framing_loss_free_control() {
    raw(Loss::None).await;
}

struct AbortOnDrop<T>(JoinHandle<T>);
impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[derive(Clone, Copy, PartialEq)]
enum CommitCase {
    Before,
    Inside,
    NoLoss,
}

fn held(latch: &gate::CommitLatch) {
    let state = latch.snapshot();
    assert!(state.installed, "actual transaction hook installed");
    assert_eq!(state.entries, 1, "exactly one actual COMMIT entry");
    assert_eq!(state.released, None, "callback still held");
    assert_eq!(state.exited, None, "callback has not exited");
}

async fn engine(f: &Fixture) {
    bounded(async {
        let mut conn = connect(&f.state.database).await.unwrap();
        let fingerprint: (String, String, String) = sqlx::query_as(
            "SELECT sqlite_version(), sqlite_source_id(), (SELECT journal_mode FROM pragma_journal_mode)",
        ).fetch_one(&mut conn).await.unwrap();
        conn.close().await.unwrap();
        assert_eq!(fingerprint, (
            "3.51.3".into(),
            "2026-03-13 10:38:09 737ae4a34738ffa0c3ff7f9bb18df914dd1cad163f28fd6b6e114a344fe6d618".into(),
            "delete".into(),
        ), "STOP: SQLite mechanism requires re-review");
    }).await;
}

async fn commit_observations(
    f: Arc<Fixture>,
    cookie: String,
    before: Arc<Gate>,
    latch: Arc<gate::CommitLatch>,
    address: SocketAddr,
    case: CommitCase,
) {
    let mut request = AbortOnDrop(tokio::spawn(
        f.app().oneshot(request(&cookie, &body(29, "viewer"))),
    ));
    bounded(before.reached()).await;
    assert!(
        latch.snapshot().installed,
        "hook installed before BeforeCommit"
    );
    assert_eq!(latch.snapshot().entries, 0);
    assert_eq!(f.state.connections.outstanding(), 1);
    if case == CommitCase::Before {
        request.0.abort();
        assert!(bounded(&mut request.0).await.unwrap_err().is_cancelled());
        // Neither gate is released: zero entry must persist through closure.
        bounded(f.state.connections.closed()).await;
        assert_eq!(f.state.connections.outstanding(), 0);
        assert_eq!(latch.snapshot().entries, 0, "no COMMIT through closure");
        assert_eq!(role(&f).await, "editor");
    } else {
        before.release();
        timeout(WAIT, latch.reached())
            .await
            .expect("actual COMMIT entry bound");
        held(&latch);
        if case == CommitCase::Inside {
            request.0.abort();
            assert!(bounded(&mut request.0).await.unwrap_err().is_cancelled());
        }
        // Cancellation is observed BEFORE release. No database operation is
        // attempted here: DELETE-mode SQLite already holds EXCLUSIVE.
        assert_eq!(f.state.connections.outstanding(), 1, "held close ticket");
        assert!(
            timeout(Duration::from_millis(100), f.state.connections.closed())
                .await
                .is_err(),
            "tracked closure must remain unacknowledged while held"
        );
        held(&latch);
        let mut stream = bounded(TcpStream::connect(address)).await.unwrap();
        bounded(stream.write_all(format!(
            "GET /rob-1111-unmatched HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
        ).as_bytes())).await.unwrap();
        let mut bytes = Vec::new();
        bounded(stream.read_to_end(&mut bytes)).await.unwrap();
        assert!(
            bytes.starts_with(b"HTTP/1.1 404 Not Found\r\n"),
            "same-app HTTP liveness"
        );
        held(&latch);
        latch.release(gate::Release::Intended);
        bounded(async {
            while latch.snapshot().exited.is_none() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await;
        assert_eq!(
            latch.snapshot().exited,
            Some(gate::Release::Intended),
            "normal release provenance"
        );
        if case == CommitCase::NoLoss {
            use http_body_util::BodyExt;
            let response = bounded(&mut request.0).await.unwrap().unwrap();
            assert_eq!(
                response.status(),
                200,
                "loss-free successful acknowledgment"
            );
            let bytes = bounded(response.into_body().collect())
                .await
                .unwrap()
                .to_bytes();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(value["kind"], "success");
            assert_eq!(value["data"]["completion"], "acknowledged");
            let doc = serde_json::to_value(super::change_role().api).unwrap();
            let mut schema = doc["paths"]["/api/memberships/role"]["post"]["responses"]["200"]
                ["content"]["application/json"]["schema"].clone();
            schema["components"] = doc["components"].clone();
            assert!(
                jsonschema::draft202012::new(&schema)
                    .unwrap()
                    .is_valid(&value),
                "complete successful acknowledgment envelope"
            );
        }
        timeout(WAIT, f.state.connections.closed())
            .await
            .expect("post-release tracked closure bound");
        assert_eq!(f.state.connections.outstanding(), 0);
        assert_eq!(latch.snapshot().entries, 1);
        assert_eq!(role(&f).await, "viewer", "exact final committed role");
    }
    bounded(async {
        let mut conn = connect(&f.state.database).await.unwrap();
        let mut tx = conn.begin_with("BEGIN IMMEDIATE").await.unwrap();
        let result =
            sqlx::query("UPDATE memberships SET role='owner' WHERE project_id=41 AND user_id=29")
                .execute(&mut *tx)
                .await
                .unwrap();
        assert_eq!(result.rows_affected(), 1, "fresh writer changed one row");
        tx.commit().await.unwrap();
        conn.close().await.unwrap();
    })
    .await;
    assert_eq!(role(&f).await, "owner", "fresh committed writer readback");
    assert_eq!(
        latch.snapshot().entries,
        usize::from(case != CommitCase::Before)
    );
}

async fn commit_case(case: CommitCase) {
    let f = Arc::new(bounded(Fixture::new()).await);
    let cookie = bounded(f.cookie(Some(11))).await;
    engine(&f).await;
    assert_eq!(role(&f).await, "editor");
    let server = Server::start(f.app()).await;
    let before = gate::hold(&f.state.database, gate::Phase::BeforeCommit);
    let hold = gate::in_commit(&f.state.database);
    // A separate task captures assertion panics, allowing explicit async cleanup
    // before propagating them. The controller guard never moves into that task.
    let mut observations = AbortOnDrop(tokio::spawn(commit_observations(
        f.clone(),
        cookie,
        before.gate.clone(),
        hold.latch.clone(),
        server.address,
        case,
    )));
    let outcome = timeout(WAIT * 3, &mut observations.0).await;
    drop(hold); // release independently of the callback Arc, before any teardown
    drop(before);
    if outcome.is_err() {
        observations.0.abort();
        let _ = timeout(WAIT, &mut observations.0).await;
    }
    let closed = timeout(WAIT, f.state.connections.closed()).await;
    server.finish().await;
    bounded(f.store.pool.close()).await;
    // Keep the original observation failure diagnostic, even for close mutants.
    outcome.expect("bounded observation task").unwrap();
    closed.expect("bounded cleanup tracked closure");
}

#[tokio::test]
async fn commit_hook_abort_before_commit() {
    commit_case(CommitCase::Before).await;
}

#[tokio::test]
async fn commit_hook_abort_inside_commit() {
    commit_case(CommitCase::Inside).await;
}

#[tokio::test]
async fn commit_hook_loss_free_acknowledgment() {
    commit_case(CommitCase::NoLoss).await;
}
