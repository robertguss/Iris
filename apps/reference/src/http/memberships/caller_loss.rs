//! Observable boundaries only: neither hold is inside SQLite's commit.
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
