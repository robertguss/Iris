#![allow(clippy::await_holding_lock)]
use super::*;
use crate::{
    domains::invitations::tests::{BOB, NOW, PROJECT, contact, database, exec, invite, issued},
    lifecycle::{self, Gate, Stopped},
    mail::tests::{Plan, Responder},
};
use axum::Router;
use sqlx::Connection;
use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering::SeqCst},
    },
    time::{Duration, Instant},
};
use tokio::{
    sync::{Notify, mpsc, oneshot},
    task::JoinHandle,
    time::timeout,
};

const WAIT: Duration = Duration::from_secs(5);

struct Running {
    address: SocketAddr,
    signal: Option<oneshot::Sender<&'static str>>,
    stopped: Option<JoinHandle<Stopped>>,
    requested: Arc<Notify>,
    gates: Vec<Arc<Gate>>,
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(signal) = self.signal.take() {
            let _ = signal.send("fixture cleanup");
        }
        for gate in &self.gates {
            gate.release();
        }
    }
}

impl Running {
    async fn start(task: Task, gates: Vec<Arc<Gate>>) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (signal, received) = oneshot::channel();
        let requested = Arc::new(Notify::new());
        let observed = requested.clone();
        let observer = Task::new("stop-observer", move |mut shutdown| async move {
            shutdown.requested().await;
            observed.notify_one();
        });
        let stopped = tokio::spawn(async move {
            lifecycle::serve(
                listener,
                Router::new(),
                vec![task, observer],
                async move { received.await.unwrap_or("fixture cleanup") },
                lifecycle::DRAIN,
            )
            .await
        });
        Self {
            address,
            signal: Some(signal),
            stopped: Some(stopped),
            requested,
            gates,
        }
    }

    async fn request_stop(&mut self) {
        if let Some(signal) = self.signal.take() {
            signal.send("SIGTERM").unwrap();
        }
        timeout(WAIT, self.requested.notified())
            .await
            .expect("stop not published");
    }

    async fn finish(mut self, connections: &Connections) {
        if self.signal.is_some() {
            self.request_stop().await;
        }
        for gate in &self.gates {
            gate.release();
        }
        let stopped = timeout(WAIT, self.stopped.take().unwrap())
            .await
            .expect("drain stalled")
            .unwrap();
        assert!(
            stopped.drained && stopped.failures.is_empty(),
            "supervised task did not drain cleanly"
        );
        timeout(lifecycle::CLOSE, connections.closed())
            .await
            .expect("tracked closure unacknowledged");
        assert!(
            tokio::net::TcpStream::connect(self.address).await.is_err(),
            "owned server still listens"
        );
    }
}

async fn line(sink: &mut mpsc::UnboundedReceiver<String>) -> String {
    timeout(WAIT, sink.recv())
        .await
        .expect("delivery diagnostic absent")
        .expect("diagnostic channel ended")
}

async fn second(conn: &mut sqlx::SqliteConnection) {
    exec(
        conn,
        "INSERT INTO users(id, display_name) VALUES(77, 'Synthetic recipient')",
    )
    .await;
    contact(conn, 77, "second@example.test").await;
    invite(conn, &BOB, PROJECT, 77, NOW).await.unwrap();
}

async fn pending(conn: &mut sqlx::SqliteConnection, claims: i64, lease: Option<i64>) {
    let row: (i64, Option<i64>, bool, bool) = sqlx::query_as("SELECT claims, lease_until, outcome IS NULL, token IS NOT NULL AND recipient_email IS NOT NULL FROM invitation_outbox ORDER BY id LIMIT 1").fetch_one(&mut *conn).await.unwrap();
    assert_eq!((row.0, row.1), (claims, lease));
    assert!(
        row.2 && row.3,
        "pending delivery lost its payload or gained an outcome"
    );
}

#[tokio::test]
async fn observed_stop_wins_over_a_ready_tick_and_suppresses_smtp_after_claim() {
    let _processes = crate::storage::shared();
    for after_claim in [false, true] {
        let (_dir, path, mut conn) = database().await;
        let token = issued(&mut conn).await;
        let message_id: String = sqlx::query_scalar("SELECT message_id FROM invitation_outbox")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        if !after_claim {
            exec(&mut conn, "UPDATE invitations SET expires_at=1000000").await;
        }
        let gate = Gate::new();
        let smtp = Responder::start(Plan::ok()).await;
        let connections = Connections::default();
        let hooks = if after_claim {
            Hooks {
                before_send: Some(gate.clone()),
                ..Hooks::default()
            }
        } else {
            Hooks {
                tick: Some(gate.clone()),
                ..Hooks::default()
            }
        };
        let mut run = Running::start(
            configured(
                path.clone(),
                connections.clone(),
                smtp.mailer(),
                || NOW,
                hooks,
            ),
            vec![gate.clone()],
        )
        .await;
        timeout(WAIT, gate.reached())
            .await
            .expect("stop gate absent");
        run.request_stop().await;
        run.finish(&connections).await;
        pending(
            &mut conn,
            i64::from(after_claim),
            after_claim.then_some(NOW + 30),
        )
        .await;
        assert_eq!(smtp.connections(), 0, "SMTP began after an observed stop");
        if after_claim {
            let (sink, mut lines) = mpsc::unbounded_channel();
            let resumed = Running::start(
                configured(
                    path,
                    connections.clone(),
                    smtp.mailer(),
                    || NOW + 30,
                    Hooks {
                        sink: Some(sink),
                        ..Hooks::default()
                    },
                ),
                vec![],
            )
            .await;
            assert!(
                line(&mut lines)
                    .await
                    .ends_with("category=accepted result=acknowledged")
            );
            resumed.finish(&connections).await;
            let messages = smtp.messages();
            assert_eq!(messages.len(), 1);
            let mail = String::from_utf8(messages[0].clone()).unwrap();
            assert!(
                mail.contains(&token) && mail.contains(&message_id),
                "recovery changed the issued credential or Message-ID"
            );
        }
        conn.close().await.unwrap();
    }
}

#[tokio::test]
async fn admitted_silent_smtp_drains_within_s18_and_no_second_claim_starts() {
    let _processes = crate::storage::shared();
    let (_dir, path, mut conn) = database().await;
    issued(&mut conn).await;
    second(&mut conn).await;
    let smtp = Responder::start(Plan {
        silent: true,
        ..Plan::ok()
    })
    .await;
    let connections = Connections::default();
    let (sink, mut lines) = mpsc::unbounded_channel();
    let mut run = Running::start(
        configured(
            path,
            connections.clone(),
            smtp.mailer(),
            || NOW,
            Hooks {
                sink: Some(sink),
                ..Hooks::default()
            },
        ),
        vec![],
    )
    .await;
    timeout(WAIT, async {
        while smtp.connections() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("SMTP not admitted");
    assert_eq!(connections.outstanding(), 1);
    let stop = Instant::now();
    run.request_stop().await;
    run.finish(&connections).await;
    assert!(
        stop.elapsed() < lifecycle::DRAIN + lifecycle::CLOSE,
        "send exceeded lifecycle bounds"
    );
    assert!(
        line(&mut lines)
            .await
            .ends_with("category=timeout result=acknowledged")
    );
    pending(&mut conn, 1, None).await;
    let retry: i64 =
        sqlx::query_scalar("SELECT next_claim_at FROM invitation_outbox ORDER BY id LIMIT 1")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert_eq!(retry, NOW + 5);
    let unclaimed: bool =
        sqlx::query_scalar("SELECT claims=0 FROM invitation_outbox ORDER BY id DESC LIMIT 1")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert!(unclaimed, "next claim started after stop");
    assert_eq!(smtp.connections(), 1);
    conn.close().await.unwrap();
}

#[tokio::test]
async fn accepted_mail_with_failed_commit_is_unconfirmed_and_stale_completion_is_distinct() {
    let _processes = crate::storage::shared();
    for commit_error in [true, false] {
        let (_dir, path, mut conn) = database().await;
        let token = issued(&mut conn).await;
        let secrets: (String, String, String) = sqlx::query_as("SELECT recipient_email, message_id, token_hash FROM invitation_outbox JOIN invitations ON invitations.id=invitation_id").fetch_one(&mut conn).await.unwrap();
        let clock = Arc::new(AtomicI64::new(NOW));
        let now = clock.clone();
        let gate = Gate::new();
        let smtp = Responder::start(Plan::ok()).await;
        let connections = Connections::default();
        let (sink, mut lines) = mpsc::unbounded_channel();
        let mut run = Running::start(
            configured(
                path,
                connections.clone(),
                smtp.mailer(),
                move || now.load(SeqCst),
                Hooks {
                    after_send: Some(gate.clone()),
                    sink: Some(sink),
                    ..Hooks::default()
                },
            ),
            vec![gate.clone()],
        )
        .await;
        timeout(WAIT, gate.reached())
            .await
            .expect("accepted SMTP not observed");
        assert_eq!(smtp.messages().len(), 1);
        if commit_error {
            exec(&mut conn, "CREATE TABLE deferred_fault(user_id INTEGER REFERENCES users(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER completion_fault AFTER UPDATE OF outcome ON invitation_outbox WHEN NEW.outcome='sent' BEGIN INSERT INTO deferred_fault VALUES(9999); END;").await;
        } else {
            clock.store(NOW + 30, SeqCst);
            run.request_stop().await;
        }
        gate.release();
        let diagnostic = line(&mut lines).await;
        let expected = if commit_error {
            "unconfirmed"
        } else {
            "no-transition"
        };
        assert!(
            diagnostic
                == format!(
                    "reference-dev: invitation-delivery outbox=1 attempt=1 stage=complete category=accepted result={expected}"
                ),
            "completion diagnostic did not preserve the result distinction"
        );
        for secret in [&token, &secrets.0, &secrets.1, &secrets.2, "250 queued"] {
            assert!(
                !diagnostic.contains(secret),
                "diagnostic disclosed delivery material"
            );
        }
        timeout(lifecycle::CLOSE, connections.closed())
            .await
            .expect("failed connection not disposed");
        pending(&mut conn, 1, Some(NOW + 30)).await;
        if commit_error {
            tokio::time::sleep(Duration::from_millis(1100)).await;
            assert_eq!(
                smtp.messages().len(),
                1,
                "uncertain acceptance immediately retransmitted"
            );
            pending(&mut conn, 1, Some(NOW + 30)).await;
            exec(&mut conn, "DROP TRIGGER completion_fault").await;
            clock.store(NOW + 30, SeqCst);
            timeout(WAIT, gate.reached())
                .await
                .expect("eligible recovery not delivered");
            assert_eq!(
                smtp.messages().len(),
                2,
                "uncertain acceptance was falsely deduplicated"
            );
            run.request_stop().await;
            gate.release();
            assert!(
                line(&mut lines)
                    .await
                    .ends_with("category=accepted result=acknowledged")
            );
            for message in smtp.messages() {
                let text = String::from_utf8(message).unwrap();
                assert!(
                    text.contains(&token) && text.contains(&secrets.1),
                    "recovery changed the issued payload"
                );
            }
        }
        run.finish(&connections).await;
        conn.close().await.unwrap();
    }
}

#[tokio::test]
async fn database_failures_dispose_connections_and_recover_on_the_next_normal_tick() {
    let _processes = crate::storage::shared();
    for sweep in [true, false] {
        let (_dir, path, mut conn) = database().await;
        issued(&mut conn).await;
        second(&mut conn).await;
        if sweep {
            exec(&mut conn, "UPDATE invitations SET expires_at=1000000 WHERE recipient_id=11; CREATE TRIGGER delivery_fault AFTER UPDATE OF outcome ON invitation_outbox BEGIN SELECT RAISE(ABORT, 'private fault'); END;").await;
        } else {
            exec(&mut conn, "CREATE TRIGGER delivery_fault AFTER UPDATE OF claims ON invitation_outbox BEGIN SELECT RAISE(ABORT, 'private fault'); END;").await;
        }
        let smtp = Responder::start(Plan::ok()).await;
        let connections = Connections::default();
        let (sink, mut lines) = mpsc::unbounded_channel();
        let run = Running::start(
            configured(
                path,
                connections.clone(),
                smtp.mailer(),
                || NOW,
                Hooks {
                    sink: Some(sink),
                    ..Hooks::default()
                },
            ),
            vec![],
        )
        .await;
        let stage = if sweep { "sweep" } else { "claim" };
        assert!(
            line(&mut lines).await
                == format!("reference-dev: invitation-delivery stage={stage} result=failed"),
            "database failure diagnostic was not bounded"
        );
        timeout(lifecycle::CLOSE, connections.closed())
            .await
            .expect("error connection retained");
        assert_eq!(smtp.connections(), 0);
        pending(&mut conn, 0, None).await;
        assert!(
            timeout(Duration::from_millis(200), lines.recv())
                .await
                .is_err(),
            "database failure retried in a tight loop"
        );
        exec(&mut conn, "DROP TRIGGER delivery_fault").await;
        assert!(
            line(&mut lines)
                .await
                .ends_with("category=accepted result=acknowledged")
        );
        run.finish(&connections).await;
        assert_eq!(smtp.messages().len(), 1);
        conn.close().await.unwrap();
    }
}

#[tokio::test]
async fn forced_fixture_failure_stops_internal_tasks_and_owned_smtp_sessions() {
    let _processes = crate::storage::shared();
    let (_dir, path, mut conn) = database().await;
    issued(&mut conn).await;
    let smtp = Responder::start(Plan {
        silent: true,
        ..Plan::ok()
    })
    .await;
    let smtp_address = smtp.addr;
    let active = smtp.active.clone();
    let connections = Connections::default();
    let mut run = Running::start(
        task(path, connections.clone(), smtp.mailer(), || NOW),
        vec![],
    )
    .await;
    let address = run.address;
    let stopped = run.stopped.take().unwrap();
    let failure = tokio::spawn(async move {
        let (_run, smtp) = (run, smtp);
        timeout(WAIT, async {
            while smtp.connections() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("cleanup control never admitted SMTP");
        assert_eq!(smtp.active.load(SeqCst), 1, "silent session was not owned");
        panic!("intentional fixture failure");
    });
    assert!(
        timeout(WAIT, failure)
            .await
            .unwrap()
            .unwrap_err()
            .is_panic()
    );
    let result = timeout(WAIT, stopped)
        .await
        .expect("internal tasks detached after fixture failure")
        .unwrap();
    assert!(result.drained && result.failures.is_empty());
    timeout(lifecycle::CLOSE, connections.closed())
        .await
        .expect("fixture connection remained open");
    timeout(WAIT, async {
        while active.load(SeqCst) != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("SMTP session leaked");
    for listener in [address, smtp_address] {
        assert!(
            tokio::net::TcpStream::connect(listener).await.is_err(),
            "fixture listener leaked"
        );
    }
    conn.close().await.unwrap();
}

#[tokio::test]
async fn supervised_task_sends_and_closes_without_holding_a_transaction() {
    let _processes = crate::storage::shared();
    let (_dir, path, mut conn) = database().await;
    issued(&mut conn).await;
    let gate = Gate::new();
    let smtp = Responder::gated(Plan::ok(), Some(gate.clone())).await;
    let connections = Connections::default();
    let run = Running::start(
        task(path, connections.clone(), smtp.mailer(), || NOW),
        vec![gate.clone()],
    )
    .await;
    timeout(WAIT, gate.reached())
        .await
        .expect("supervised delivery never reached SMTP");
    assert_eq!(
        connections.outstanding(),
        1,
        "send bypassed tracked connection"
    );
    // This independent connection must acquire and commit a writer during SMTP.
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .expect("transaction held across SMTP");
    sqlx::query("UPDATE projects SET name=name WHERE id=43")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    gate.release();
    timeout(WAIT, async {
        loop {
            let sent: bool = sqlx::query_scalar("SELECT outcome='sent' FROM invitation_outbox")
                .fetch_one(&mut conn)
                .await
                .unwrap_or(false);
            if sent {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("delivery never completed");
    let cleared: bool = sqlx::query_scalar("SELECT claims=1 AND token IS NULL AND recipient_email IS NULL AND lease_until IS NULL FROM invitation_outbox").fetch_one(&mut conn).await.unwrap();
    assert!(
        cleared,
        "terminal delivery retained credential, address or lease"
    );
    assert_eq!(smtp.messages().len(), 1);
    run.finish(&connections).await;
    conn.close().await.unwrap();
}
