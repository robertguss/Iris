#![cfg(feature = "dev-identity")]
use axum::{body::Body, http::Request};
use iris_api_spike::{
    AppState, delivery::Mailer, development_identity, seed_demo, unix_time, utoipa_router,
};
use iris_sqlite_spike::{
    connect, migrate,
    outbox::{self, Completion},
};
use serde_json::Value;
use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    time::Duration,
};
use tower::ServiceExt;

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
#[ignore = "requires pinned Mailpit; run explicitly in CI after fixture installation"]
async fn smtp_failure_recovery_and_ambiguous_success() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("app.db");
    let mut conn = connect(&database).await.unwrap();
    migrate(&mut conn).await.unwrap();
    seed_demo(&mut conn, unix_time()).await.unwrap();
    let smtp = TcpListener::bind("127.0.0.1:0").unwrap();
    let http = TcpListener::bind("127.0.0.1:0").unwrap();
    let smtp_port = smtp.local_addr().unwrap().port();
    let http_port = http.local_addr().unwrap().port();
    drop(smtp);
    let mailer = Mailer::new("http://127.0.0.1:5174".into(), smtp_port).unwrap();
    let app = development_identity(utoipa_router().0).with_state(AppState {
        database: database.clone(),
        now: unix_time,
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/invitations")
                .header("content-type", "application/json")
                .header("x-iris-dev-user", "11")
                .body(Body::from(r#"{"project_id":"41","recipient_id":"29"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        201,
        "SMTP is unavailable but enqueue succeeds"
    );
    mailer.tick(&database).await.unwrap();
    let state: (String, i64, String) =
        sqlx::query_as("SELECT state,attempts,reason FROM invitation_outbox")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert_eq!(state, ("pending".into(), 1, "transient_failure".into()));
    drop(http);
    let binary = std::env::var("MAILPIT_BIN")
        .unwrap_or_else(|_| format!("{}/.local/bin/mailpit", std::env::var("HOME").unwrap()));
    let _process = Process(
        Command::new(binary)
            .args([
                "--listen",
                &format!("127.0.0.1:{http_port}"),
                "--smtp",
                &format!("127.0.0.1:{smtp_port}"),
                "--disable-version-check",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{http_port}");
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if client
                .get(format!("{base}/livez"))
                .send()
                .await
                .is_ok_and(|r| r.status().is_success())
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    // Advance only the retry schedule; retain the same payload and attempt history.
    sqlx::query("UPDATE invitation_outbox SET next_attempt_at=?")
        .bind(unix_time())
        .execute(&mut conn)
        .await
        .unwrap();
    let first = outbox::claim(&mut conn, unix_time())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(mailer.send(&first).await, Completion::Sent));
    // SMTP accepted, but worker dies before acknowledgement. Reclaim expired lease.
    sqlx::query("UPDATE invitation_outbox SET lease_until=0")
        .execute(&mut conn)
        .await
        .unwrap();
    let second = outbox::claim(&mut conn, unix_time())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.token, first.token);
    assert!(matches!(mailer.send(&second).await, Completion::Sent));
    assert!(
        !outbox::complete(&mut conn, &first, Completion::Sent, unix_time())
            .await
            .unwrap()
    );
    assert!(
        outbox::complete(&mut conn, &second, Completion::Sent, unix_time())
            .await
            .unwrap()
    );
    let list: Value = client
        .get(format!("{base}/api/v1/messages"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let messages = list["messages"].as_array().unwrap();
    assert_eq!(
        messages.len(),
        2,
        "SMTP does not guarantee exactly-once delivery"
    );
    let mut ids = Vec::new();
    for message in messages {
        let id = message["ID"].as_str().unwrap();
        let mail: Value = client
            .get(format!("{base}/api/v1/message/{id}"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(mail["To"][0]["Address"], "bob@example.test");
        assert!(
            mail["Text"]
                .as_str()
                .unwrap()
                .starts_with("Local Iris experiment — no real email was sent.")
        );
        assert!(mail["Text"].as_str().unwrap().contains(&format!(
            "http://127.0.0.1:5174/#invitation={}",
            second.token
        )));
        let headers: Value = client
            .get(format!("{base}/api/v1/message/{id}/headers"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        ids.push(headers["Message-Id"].clone());
    }
    assert!(!ids[0].is_null());
    assert_eq!(ids[0], ids[1]);
    let retained: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM invitation_outbox WHERE token IS NOT NULL OR recipient IS NOT NULL",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(retained, 0);
}
