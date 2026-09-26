use super::*;
use crate::auth::store::Store;
use action::{ChangeRole, Cleanup, Stage};
use axum::{body::Body, http::Request as HttpRequest};
use http_body_util::BodyExt;
use iris_sqlite_spike::{MemberRole, connect, migrate};
use sqlx::Connection;
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
};
use tower::ServiceExt;
use tower_sessions::{Session, SessionStore, session::Record};

const PATH: &str = "/api/memberships/role";
const ORIGIN: &str = "http://127.0.0.1:5173";

struct Fixture {
    _dir: tempfile::TempDir,
    provider: Child,
    state: AppState,
    auth: Auth,
    store: Store,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.provider.kill();
        let _ = self.provider.wait();
    }
}
impl Fixture {
    async fn new() -> Self {
        let script = format!(
            "import {{startOidcProvider}} from '{}'; const p=await startOidcProvider({{port:0,redirectUri:'{ORIGIN}/api/auth/callback',testControls:true}}); console.log(p.issuer);",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../checks/oidc-provider.mjs")
                .display()
        );
        let mut provider = Command::new("node")
            .args(["--input-type=module", "-e", &script])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut issuer = String::new();
        BufReader::new(provider.stdout.take().unwrap())
            .read_line(&mut issuer)
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("s16.db");
        let mut conn = connect(&database).await.unwrap();
        migrate(&mut conn).await.unwrap();
        crate::seed_demo(&mut conn, crate::unix_time())
            .await
            .unwrap();
        sqlx::query("INSERT INTO memberships VALUES (41,29,'editor')")
            .execute(&mut conn)
            .await
            .unwrap();
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(&database)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        let store = Store {
            pool,
            now: crate::unix_time,
        };
        store.migrate().await.unwrap();
        let auth = Auth::discover(
            store.clone(),
            ORIGIN.into(),
            issuer.trim().into(),
            "iris-local".into(),
            None,
        )
        .await
        .unwrap();
        Self {
            _dir: dir,
            provider,
            state: AppState {
                database,
                now: crate::unix_time,
            },
            auth,
            store,
        }
    }
    fn app(&self) -> Router {
        authenticated(self.auth.clone()).with_state(self.state.clone())
    }
    async fn cookie(&self, user: Option<i64>) -> String {
        let mut record = Record {
            id: Default::default(),
            data: [(
                "browser".into(),
                json!({"csrf":"csrf-canary", "user_id":user,"expires_at":crate::unix_time()+3600}),
            )]
            .into(),
            expiry_date: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
        };
        self.store.create(&mut record).await.unwrap();
        format!("iris-session-dev={}", record.id)
    }
    async fn role(&self, user: i64) -> String {
        sqlx::query_scalar("SELECT role FROM memberships WHERE project_id=41 AND user_id=?")
            .bind(user)
            .fetch_one(&self.store.pool)
            .await
            .unwrap()
    }
}

fn request(cookie: &str, body: &str) -> HttpRequest<Body> {
    HttpRequest::builder()
        .method("POST")
        .uri(PATH)
        .header("content-type", "application/json")
        .header("cookie", cookie)
        .header("origin", ORIGIN)
        .header("x-iris-csrf", "csrf-canary")
        .header("x-request-id", "caller-secret-canary")
        .body(Body::from(body.to_owned()))
        .unwrap()
}
fn body(user: i64, role: &str) -> String {
    json!({"project_id":"41", "user_id":user.to_string(),"role":role}).to_string()
}
async fn collect(response: Response) -> (u16, Value) {
    let status = response.status().as_u16();
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(!value.to_string().contains("canary"));
    (status, value)
}
fn validate(status: u16, body: &Value) -> bool {
    let doc = serde_json::to_value(routes().1).unwrap();
    let mut schema =
        doc["paths"][PATH]["post"]["responses"][status.to_string()]["content"]["application/json"]
            ["schema"]
            .clone();
    schema["components"] = doc["components"].clone();
    jsonschema::draft202012::new(&schema)
        .unwrap()
        .is_valid(body)
}
fn expect(response: &(u16, Value), status: u16, kind: &str, code: Option<&str>) {
    assert_eq!(response.0, status, "{response:?}");
    assert_eq!(response.1["kind"], kind);
    assert_eq!(response.1.get("code").and_then(Value::as_str), code);
    assert!(
        validate(status, &response.1),
        "schema rejected {response:?}"
    );
}

#[test]
fn independent_contract() {
    let doc = serde_json::to_value(routes().1).unwrap();
    assert_eq!(doc["openapi"], "3.1.0");
    assert_eq!(
        doc["paths"].as_object().unwrap().keys().collect::<Vec<_>>(),
        [PATH]
    );
    let op = &doc["paths"][PATH]["post"];
    assert_eq!(op["operationId"], "changeMemberRole");
    assert_eq!(op["x-iris"]["operation"], "memberships.change_role");
    assert_eq!(op["x-iris"]["recovery"]["replay"], false);
    assert_eq!(
        op["x-iris"]["prerequisites"]["memberships.last_owner"],
        "memberships.another_owner_required"
    );
    let mut actual = vec![];
    for (status, response) in op["responses"].as_object().unwrap() {
        for branch in response["content"]["application/json"]["schema"]["oneOf"]
            .as_array()
            .unwrap()
        {
            actual.push((
                status.as_str(),
                branch["properties"]["kind"]["enum"][0].as_str().unwrap(),
                branch["properties"]["code"]["enum"][0]
                    .as_str()
                    .unwrap_or(""),
            ));
        }
    }
    let mut expected = vec![
        ("200", "success", ""),
        ("400", "refused", "http.invalid_request"),
        ("401", "refused", "http.unauthenticated"),
        ("403", "refused", "http.csrf_refused"),
        ("403", "rejected", "memberships.forbidden"),
        ("404", "rejected", "memberships.member_not_found"),
        ("409", "rejected", "memberships.last_owner"),
        ("500", "failure", "iris.internal"),
        ("503", "failure", "iris.unavailable"),
    ];
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
    assert!(!doc.to_string().contains("memberships.at_least_one_owner"));
    let last = json!({"schema_version":1,"operation":"memberships.change_role","request_id":"req_00000000000000000000000000000000","kind":"rejected","code":"memberships.last_owner","message":"safe"});
    assert!(validate(409, &last));
    for (field, value) in [
        ("code", json!("memberships.forbidden")),
        ("kind", json!("refused")),
        ("schema_version", json!(2)),
        ("operation", json!("other")),
    ] {
        let mut wrong = last.clone();
        wrong[field] = value;
        assert!(!validate(409, &wrong));
    }
}

#[tokio::test]
async fn whole_request_contract_and_fixtures() {
    let f = Fixture::new().await;
    let alice = f.cookie(Some(11)).await;
    let bob = f.cookie(Some(29)).await;
    let anonymous = f.cookie(None).await;
    let mut fixtures = vec![];
    for (cookie, target, role, status, kind, code) in [
        (&alice, 29, "viewer", 200, "success", None),
        (&alice, 29, "viewer", 200, "success", None),
        (
            &bob,
            999,
            "editor",
            403,
            "rejected",
            Some("memberships.forbidden"),
        ),
        (
            &alice,
            999,
            "editor",
            404,
            "rejected",
            Some("memberships.member_not_found"),
        ),
        (
            &alice,
            11,
            "editor",
            409,
            "rejected",
            Some("memberships.last_owner"),
        ),
        (
            &anonymous,
            29,
            "owner",
            401,
            "refused",
            Some("http.unauthenticated"),
        ),
    ] {
        let response = collect(
            f.app()
                .oneshot(request(cookie, &body(target, role)))
                .await
                .unwrap(),
        )
        .await;
        expect(&response, status, kind, code);
        fixtures.push(response);
    }
    assert_eq!(f.role(29).await, "viewer");
    for header in ["origin", "x-iris-csrf"] {
        let mut req = request(&alice, &body(29, "owner"));
        req.headers_mut().remove(header);
        let response = collect(f.app().oneshot(req).await.unwrap()).await;
        expect(&response, 403, "refused", Some("http.csrf_refused"));
        fixtures.push(response);
    }
    let mut invalid = vec![
        "{".into(),
        json!({"project_id":"41","user_id":"29","role":"owner","actor_id":11}).to_string(),
        body(29, "bogus"),
    ];
    for id in ["0", "01", "-1", "9223372036854775808"] {
        invalid.push(json!({"project_id":id,"user_id":"29","role":"owner"}).to_string());
    }
    // Valid JSON and an otherwise valid mutation: without the body limit this
    // would succeed, rather than also failing ordinary JSON parsing.
    invalid.push(format!(
        "{}{}",
        " ".repeat(2 * 1024 * 1024),
        body(29, "owner")
    ));
    for text in invalid {
        let response = collect(f.app().oneshot(request(&alice, &text)).await.unwrap()).await;
        expect(&response, 400, "refused", Some("http.invalid_request"));
        fixtures.push(response);
    }
    let mut req = request(&alice, &body(29, "owner"));
    req.headers_mut()
        .insert("content-type", "text/plain".parse().unwrap());
    let response = collect(f.app().oneshot(req).await.unwrap()).await;
    expect(&response, 400, "refused", Some("http.invalid_request"));
    fixtures.push(response);
    assert_eq!(
        f.role(29).await,
        "viewer",
        "refusals must not dispatch mutation"
    );
    let mut conn = connect(&f.state.database).await.unwrap();
    let tx = conn.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let response = collect(
        f.app()
            .oneshot(request(&alice, &body(29, "owner")))
            .await
            .unwrap(),
    )
    .await;
    expect(&response, 503, "failure", Some("iris.unavailable"));
    fixtures.push(response);
    tx.rollback().await.unwrap();
    sqlx::raw_sql("CREATE TRIGGER body_fault AFTER UPDATE ON memberships BEGIN SELECT RAISE(FAIL,'sql-secret-canary'); END;").execute(&f.store.pool).await.unwrap();
    let response = collect(
        f.app()
            .oneshot(request(&alice, &body(29, "owner")))
            .await
            .unwrap(),
    )
    .await;
    expect(&response, 500, "failure", Some("iris.internal"));
    fixtures.push(response);
    assert_eq!(f.role(29).await, "viewer");
    let ids = fixtures
        .iter()
        .map(|(_, v)| v["request_id"].as_str().unwrap())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), fixtures.len());
    if let Ok(path) = std::env::var("IRIS_S16_FIXTURES") {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(
                &fixtures
                    .iter()
                    .map(|(s, b)| json!({"status":s,"body":b}))
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
        )
        .unwrap();
    }
}

#[tokio::test]
async fn session_save_failure_after_commit() {
    let f = Fixture::new().await;
    let alice = f.cookie(Some(11)).await;
    sqlx::raw_sql("CREATE TRIGGER fail_save BEFORE UPDATE ON iris_sessions BEGIN SELECT RAISE(FAIL,'session-secret-canary'); END;").execute(&f.store.pool).await.unwrap();
    // Control: ordinary membership does not modify the session or call save.
    let response = collect(
        f.app()
            .oneshot(request(&alice, &body(29, "viewer")))
            .await
            .unwrap(),
    )
    .await;
    expect(&response, 200, "success", None);
    let reached = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = reached.clone();
    let inner = routes().0.layer(middleware::from_fn(
        move |session: Session, req: Request, next: Next| {
            let flag = flag.clone();
            async move {
                let response = next.run(req).await;
                assert_eq!(response.status(), StatusCode::OK);
                session.insert("post_commit_test", true).await.unwrap();
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                response
            }
        },
    ));
    let app = boundary(f.auth.clone().layer(inner)).with_state(f.state.clone());
    let response = collect(
        app.oneshot(request(&alice, &body(29, "owner")))
            .await
            .unwrap(),
    )
    .await;
    expect(&response, 500, "failure", Some("iris.internal"));
    assert!(reached.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(
        f.role(29).await,
        "owner",
        "public failure does not imply no commit"
    );
    // The injected key was not saved: the real Store::save SQL failed.
    let data: String = sqlx::query_scalar("SELECT data FROM iris_sessions")
        .fetch_one(&f.store.pool)
        .await
        .unwrap();
    assert!(!data.contains("post_commit_test"));
}

#[tokio::test]
async fn connection_and_session_load_failures_are_request_failures() {
    let f = Fixture::new().await;
    let alice = f.cookie(Some(11)).await;
    let app = authenticated(f.auth.clone()).with_state(AppState {
        database: f._dir.path().join("missing-directory/db"),
        now: crate::unix_time,
    });
    let response = collect(
        app.oneshot(request(&alice, &body(29, "owner")))
            .await
            .unwrap(),
    )
    .await;
    expect(&response, 500, "failure", Some("iris.internal"));
    assert_eq!(f.role(29).await, "editor");
    sqlx::query("DROP TABLE iris_sessions")
        .execute(&f.store.pool)
        .await
        .unwrap();
    let response = collect(
        f.app()
            .oneshot(request(&alice, &body(29, "owner")))
            .await
            .unwrap(),
    )
    .await;
    expect(&response, 500, "failure", Some("iris.internal"));
    assert_eq!(f.role(29).await, "editor");
}

#[tokio::test]
async fn refusal_rewrite_preserves_cookie_headers() {
    let router = Router::new().route(
        PATH,
        axum::routing::post(|| async {
            let mut response = crate::ApiError(ErrorCode::Csrf).into_response();
            response
                .headers_mut()
                .insert("cache-control", "no-store".parse().unwrap());
            response
                .headers_mut()
                .append("set-cookie", "one=1; HttpOnly".parse().unwrap());
            response
                .headers_mut()
                .append("set-cookie", "two=2; HttpOnly".parse().unwrap());
            response
                .headers_mut()
                .insert("content-length", "0".parse().unwrap());
            response
        }),
    );
    let response = boundary(router)
        .with_state(AppState {
            database: "unused".into(),
            now: crate::unix_time,
        })
        .oneshot(request("", "{}"))
        .await
        .unwrap();
    assert_eq!(response.headers().get_all("set-cookie").iter().count(), 2);
    let response = collect(response).await;
    expect(&response, 403, "refused", Some("http.csrf_refused"));
}

#[tokio::test]
async fn real_body_error_rolls_back_and_concurrent_demotions_serialize() {
    let f = Fixture::new().await;
    let mut conn = connect(&f.state.database).await.unwrap();
    sqlx::raw_sql("CREATE TRIGGER fail_body AFTER UPDATE ON memberships BEGIN SELECT RAISE(FAIL,'private'); END;").execute(&mut conn).await.unwrap();
    // Control: FAIL keeps the statement's update pending until explicit rollback.
    let mut tx = conn.begin_with("BEGIN IMMEDIATE").await.unwrap();
    assert!(
        sqlx::query("UPDATE memberships SET role='owner' WHERE project_id=41 AND user_id=29")
            .execute(&mut *tx)
            .await
            .is_err()
    );
    let role: String =
        sqlx::query_scalar("SELECT role FROM memberships WHERE project_id=41 AND user_id=29")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(role, "owner");
    tx.rollback().await.unwrap();
    let command = || ChangeRole {
        project_id: 41,
        user_id: 29,
        role: MemberRole::Owner,
    };
    let error = action::change_role(&mut conn, &Actor(11), command())
        .await
        .unwrap_err();
    assert_eq!(
        error,
        ActionError::Failed {
            primary: StopReason::Execution {
                stage: Stage::Body,
                kind: FailureKind::Other
            },
            cleanup: Cleanup::RollbackAcknowledged
        }
    );
    assert_eq!(f.role(29).await, "editor");
    sqlx::query("DROP TRIGGER fail_body")
        .execute(&mut conn)
        .await
        .unwrap();
    action::change_role(&mut conn, &Actor(11), command())
        .await
        .unwrap();
    let mut second = connect(&f.state.database).await.unwrap();
    sqlx::query("PRAGMA busy_timeout=5000")
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query("PRAGMA busy_timeout=5000")
        .execute(&mut second)
        .await
        .unwrap();
    let barrier = tokio::sync::Barrier::new(2);
    let (a, b) = tokio::join!(
        async {
            barrier.wait().await;
            action::change_role(
                &mut conn,
                &Actor(11),
                ChangeRole {
                    project_id: 41,
                    user_id: 11,
                    role: MemberRole::Editor,
                },
            )
            .await
        },
        async {
            barrier.wait().await;
            action::change_role(
                &mut second,
                &Actor(29),
                ChangeRole {
                    project_id: 41,
                    user_id: 29,
                    role: MemberRole::Viewer,
                },
            )
            .await
        }
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let error = if let Err(e) = a { e } else { b.unwrap_err() };
    assert_eq!(error, ActionError::Rejected(Rejection::LastOwner));
    let owners: i64 =
        sqlx::query_scalar("SELECT count(*) FROM memberships WHERE project_id=41 AND role='owner'")
            .fetch_one(&f.store.pool)
            .await
            .unwrap();
    assert_eq!(owners, 1);
}

#[tokio::test]
async fn unclassified_responses_stay_unclassified() {
    for marked in [false, true] {
        let router = Router::new().route(
            PATH,
            axum::routing::post(move || async move {
                if marked {
                    crate::ApiError(ErrorCode::Forbidden).into_response()
                } else {
                    StatusCode::FORBIDDEN.into_response()
                }
            }),
        );
        let response = boundary(router)
            .with_state(AppState {
                database: "unused".into(),
                now: crate::unix_time,
            })
            .oneshot(request("", "{}"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert!(!String::from_utf8_lossy(&bytes).contains("memberships.change_role"));
    }
    let f = Fixture::new().await;
    for (method, path, status) in [("GET", PATH, 405), ("POST", "/absent", 404)] {
        let response = f
            .app()
            .oneshot(
                HttpRequest::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert!(!String::from_utf8_lossy(&bytes).contains("memberships.change_role"));
    }
}

#[tokio::test]
async fn failed_cleanup_never_projects_rejection() {
    for primary in [
        StopReason::Rejected(Rejection::LastOwner),
        StopReason::Execution {
            stage: Stage::Body,
            kind: FailureKind::Other,
        },
    ] {
        let response = reply(
            &RequestId("req_00000000000000000000000000000000".into()),
            Err(ActionError::Failed {
                primary,
                cleanup: Cleanup::Unconfirmed {
                    rollback_error: Some(FailureKind::Other),
                },
            }),
        );
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["code"], "iris.internal");
        assert!(!body.to_string().contains("last_owner"));
        assert!(validate(500, &body));
    }
}

#[tokio::test]
async fn begin_and_commit_errors_do_not_claim_cleanup() {
    let f = Fixture::new().await;
    let mut locked = connect(&f.state.database).await.unwrap();
    let mut conn = connect(&f.state.database).await.unwrap();
    let tx = locked.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let command = || ChangeRole {
        project_id: 41,
        user_id: 29,
        role: MemberRole::Owner,
    };
    assert_eq!(
        action::change_role(&mut conn, &Actor(11), command())
            .await
            .unwrap_err(),
        ActionError::Failed {
            primary: StopReason::Execution {
                stage: Stage::Begin,
                kind: FailureKind::Busy
            },
            cleanup: Cleanup::Unconfirmed {
                rollback_error: None
            }
        }
    );
    tx.rollback().await.unwrap();
    // A real deferred FK violation fails COMMIT rather than the body UPDATE.
    sqlx::raw_sql("CREATE TABLE deferred_fault (user_id INTEGER REFERENCES users(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER commit_fault AFTER UPDATE ON memberships BEGIN INSERT INTO deferred_fault VALUES (9999); END;").execute(&mut conn).await.unwrap();
    assert_eq!(
        action::change_role(&mut conn, &Actor(11), command())
            .await
            .unwrap_err(),
        ActionError::Failed {
            primary: StopReason::Execution {
                stage: Stage::Commit,
                kind: FailureKind::Other
            },
            cleanup: Cleanup::Unconfirmed {
                rollback_error: None
            }
        }
    );
    // Explicit disposal, not a rollback-acknowledgment assertion. This test
    // covers this SQLite failure, not arbitrary commit ambiguity/task loss.
    conn.close().await.unwrap();
    assert_eq!(f.role(29).await, "editor");
}
