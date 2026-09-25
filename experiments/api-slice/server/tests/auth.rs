use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request},
};
use http_body_util::BodyExt;
use iris_api_spike::{
    AppState,
    auth::{Auth, store::Store},
    seed_demo, unix_time, utoipa_router,
};
use openidconnect::reqwest;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
};
use tower::ServiceExt;
use tower_sessions::{
    SessionStore,
    session::{Id, Record},
};

#[path = "../../../agent-interface/evidence.rs"]
mod evidence;

struct Fixture {
    _dir: tempfile::TempDir,
    provider: Child,
    issuer: String,
    app: Router,
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
        let provider_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../checks/oidc-provider.mjs");
        let script = format!(
            "import {{startOidcProvider}} from '{}'; const p=await startOidcProvider({{port:0,redirectUri:'http://127.0.0.1:5173/api/auth/callback',testControls:true}}); console.log(p.issuer);",
            provider_path.display()
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
        let issuer = issuer.trim().to_owned();
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("app.db");
        let mut conn = iris_sqlite_spike::connect(&database).await.unwrap();
        iris_sqlite_spike::migrate(&mut conn).await.unwrap();
        seed_demo(&mut conn, unix_time()).await.unwrap();
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(&database)
                    .foreign_keys(true)
                    .busy_timeout(std::time::Duration::from_secs(1)),
            )
            .await
            .unwrap();
        let store = Store {
            pool,
            now: unix_time,
        };
        store.migrate().await.unwrap();
        for (sub, user) in [("alice", 11_i64), ("bob", 29)] {
            sqlx::query("INSERT INTO iris_external_identities VALUES(?,?,?)")
                .bind(&issuer)
                .bind(sub)
                .bind(user)
                .execute(&store.pool)
                .await
                .unwrap();
        }
        let auth = Auth::discover(
            store.clone(),
            "http://127.0.0.1:5173".into(),
            issuer.clone(),
            "iris-local".into(),
            None,
        )
        .await
        .unwrap();
        let app = auth.layer(utoipa_router().0).with_state(AppState {
            database,
            now: unix_time,
        });
        Self {
            _dir: dir,
            provider,
            issuer,
            app,
            store,
        }
    }
}

#[derive(Default)]
struct Browser {
    cookie: String,
    csrf: String,
}
async fn send(
    app: &Router,
    b: &mut Browser,
    method: &str,
    path: &str,
    body: Value,
    origin: Option<&str>,
    csrf: bool,
) -> (u16, HeaderMap, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("x-iris-dev-user", "11");
    if !b.cookie.is_empty() {
        request = request.header("cookie", &b.cookie);
    }
    if let Some(origin) = origin {
        request = request.header("origin", origin);
    }
    if csrf {
        request = request.header("x-iris-csrf", &b.csrf);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    for cookie in headers.get_all("set-cookie") {
        b.cookie = cookie
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
    }
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    assert_eq!(headers["cache-control"], "no-store");
    if !value.is_null() {
        let spec = serde_json::to_value(utoipa_router().1).unwrap();
        let path = path.split('?').next().unwrap();
        let mut schema = spec["paths"][path][method.to_lowercase()]["responses"]
            [status.to_string()]["content"]["application/json"]["schema"]
            .clone();
        assert!(!schema.is_null(), "undocumented {method} {path} {status}");
        schema["components"] = spec["components"].clone();
        jsonschema::validate(&schema, &value).unwrap();
    }
    (status, headers, value)
}
async fn bootstrap(f: &Fixture, b: &mut Browser) -> Value {
    let (status, _, v) = send(
        &f.app,
        b,
        "GET",
        "/api/auth/session",
        Value::Null,
        None,
        false,
    )
    .await;
    assert_eq!(status, 200);
    b.csrf = v["csrf_token"].as_str().unwrap().into();
    v
}
async fn start(f: &Fixture, b: &mut Browser) -> reqwest::Url {
    let (status, _, v) = send(
        &f.app,
        b,
        "POST",
        "/api/auth/login",
        Value::Null,
        Some("http://127.0.0.1:5173"),
        true,
    )
    .await;
    assert_eq!(status, 200);
    reqwest::Url::parse(v["authorization_url"].as_str().unwrap()).unwrap()
}
async fn authorize(url: &reqwest::Url, identity: &str) -> String {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let mut params: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    params.push(("identity".into(), identity.into()));
    let response = client
        .post(format!("{}/authorize", url.origin().ascii_serialization()))
        .form(&params)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 303);
    let callback = reqwest::Url::parse(response.headers()["location"].to_str().unwrap()).unwrap();
    format!("{}?{}", callback.path(), callback.query().unwrap())
}
async fn login(f: &Fixture, b: &mut Browser, identity: &str) {
    bootstrap(f, b).await;
    let old = b.cookie.clone();
    let csrf = b.csrf.clone();
    let url = start(f, b).await;
    let callback = authorize(&url, identity).await;
    assert_eq!(
        send(&f.app, b, "GET", &callback, Value::Null, None, false)
            .await
            .0,
        303
    );
    assert_ne!(b.cookie, old);
    bootstrap(f, b).await;
    assert_ne!(b.csrf, csrf);
}

#[tokio::test]
async fn session_login_csrf_permissions_logout_and_no_resurrection() {
    let f = Fixture::new().await;
    let mut a = Browser::default();
    let (status, headers, value) = send(
        &f.app,
        &mut a,
        "GET",
        "/api/auth/session",
        Value::Null,
        None,
        false,
    )
    .await;
    assert_eq!(status, 200);
    assert!(value["user_id"].is_null());
    a.csrf = value["csrf_token"].as_str().unwrap().into();
    let cookie = headers["set-cookie"].to_str().unwrap();
    assert!(
        cookie.contains("HttpOnly") && cookie.contains("SameSite=Lax") && cookie.contains("Path=/")
    );
    assert!(!cookie.contains("Secure"));
    let issue = json!({"project_id":"41","recipient_id":"29"});
    for (origin, csrf) in [
        (None, true),
        (Some("null"), true),
        (Some("https://evil.example"), true),
        (Some("http://127.0.0.1:5173"), false),
    ] {
        let (status, _, v) = send(
            &f.app,
            &mut a,
            "POST",
            "/api/invitations",
            issue.clone(),
            origin,
            csrf,
        )
        .await;
        assert_eq!(status, 403);
        assert_eq!(v["code"], "csrf");
    }
    // Even with the development feature compiled, fake headers do not sign in.
    assert_eq!(
        send(
            &f.app,
            &mut a,
            "POST",
            "/api/invitations",
            issue.clone(),
            Some("http://127.0.0.1:5173"),
            true
        )
        .await
        .0,
        401
    );
    login(&f, &mut a, "alice").await;
    let first_id = a.cookie.split('=').nth(1).unwrap().parse::<Id>().unwrap();
    let stale = f.store.load(&first_id).await.unwrap().unwrap();
    assert!((stale.expiry_date.unix_timestamp() - unix_time() - 28800).abs() < 5);
    let (status, h, v) = send(
        &f.app,
        &mut a,
        "GET",
        "/api/auth/session",
        Value::Null,
        None,
        false,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(v["user_id"], "11");
    assert!(
        !h.contains_key("set-cookie"),
        "read-only requests should not refresh sessions"
    );
    let mut second = Browser::default();
    login(&f, &mut second, "alice").await;
    let (_, _, invitation) = send(
        &f.app,
        &mut a,
        "POST",
        "/api/invitations",
        issue,
        Some("http://127.0.0.1:5173"),
        true,
    )
    .await;
    let mut bob = Browser::default();
    login(&f, &mut bob, "bob").await;
    let forbidden = send(
        &f.app,
        &mut bob,
        "POST",
        "/api/invitations",
        json!({"project_id":"41","recipient_id":"11"}),
        Some("http://127.0.0.1:5173"),
        true,
    )
    .await;
    assert_eq!(forbidden.0, 403);
    assert_eq!(forbidden.2["code"], "forbidden");
    let token = json!({"token":invitation["token"]});
    assert_eq!(
        send(
            &f.app,
            &mut a,
            "POST",
            "/api/invitations/accept",
            token.clone(),
            Some("http://127.0.0.1:5173"),
            true
        )
        .await
        .0,
        404
    );
    let accepted = send(
        &f.app,
        &mut bob,
        "POST",
        "/api/invitations/accept",
        token,
        Some("http://127.0.0.1:5173"),
        true,
    )
    .await;
    assert_eq!(accepted.0, 200);
    assert_eq!(accepted.2, json!({"project_id":"41","user_id":"29"}));
    for (path, body) in [
        (
            "/api/memberships/role",
            json!({"project_id":"41","user_id":"29","role":"viewer"}),
        ),
        (
            "/api/memberships/remove",
            json!({"project_id":"41","user_id":"29"}),
        ),
    ] {
        let denied = send(
            &f.app,
            &mut bob,
            "POST",
            path,
            body.clone(),
            Some("http://127.0.0.1:5173"),
            true,
        )
        .await;
        assert_eq!(denied.0, 403);
        assert_eq!(denied.2["code"], "forbidden");
        for (origin, csrf) in [
            (Some("http://127.0.0.1:5173"), false),
            (Some("https://evil.example"), true),
        ] {
            let denied = send(&f.app, &mut a, "POST", path, body.clone(), origin, csrf).await;
            assert_eq!(denied.0, 403);
            assert_eq!(denied.2["code"], "csrf");
        }
        let changed = send(
            &f.app,
            &mut a,
            "POST",
            path,
            body,
            Some("http://127.0.0.1:5173"),
            true,
        )
        .await;
        assert_eq!(changed.0, 200);
        assert_eq!(
            changed.2,
            json!({"project_id":"41","user_id":"29","role": if path.ends_with("role") { json!("viewer") } else { Value::Null }})
        );
    }
    assert_eq!(
        send(
            &f.app,
            &mut a,
            "POST",
            "/api/auth/logout",
            Value::Null,
            Some("http://127.0.0.1:5173"),
            true
        )
        .await
        .0,
        204
    );
    assert!(f.store.save(&stale).await.is_err());
    assert!(f.store.load(&first_id).await.unwrap().is_none());
    assert!(bootstrap(&f, &mut a).await["user_id"].is_null());
    assert_eq!(bootstrap(&f, &mut second).await["user_id"], "11");
    let id = second.cookie.split('=').nth(1).unwrap();
    sqlx::query("UPDATE iris_sessions SET expires_at=? WHERE id=?")
        .bind(unix_time())
        .bind(id)
        .execute(&f.store.pool)
        .await
        .unwrap();
    assert!(bootstrap(&f, &mut second).await["user_id"].is_null());
}

#[tokio::test]
async fn oidc_verifies_claims_and_consumes_attempts_once() {
    let f = Fixture::new().await;
    let http = reqwest::Client::new();
    let mut b = Browser::default();
    bootstrap(&f, &mut b).await;
    for fault in [
        "wrong_signature",
        "issuer",
        "audience",
        "nonce",
        "expired",
        "missing_id_token",
    ] {
        let url = start(&f, &mut b).await;
        let callback = authorize(&url, "alice").await;
        http.post(format!("{}/__test/next-id-token-fault", f.issuer))
            .json(&json!({"fault":fault}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        assert_eq!(
            send(&f.app, &mut b, "GET", &callback, Value::Null, None, false)
                .await
                .0,
            401,
            "{fault}"
        );
        assert!(bootstrap(&f, &mut b).await["user_id"].is_null());
    }
    let url = start(&f, &mut b).await;
    let callback = authorize(&url, "alice").await;
    let mut other = Browser::default();
    bootstrap(&f, &mut other).await;
    assert_eq!(
        send(
            &f.app,
            &mut other,
            "GET",
            &callback,
            Value::Null,
            None,
            false
        )
        .await
        .0,
        401
    );
    let before: Value = http
        .get(format!("{}/__test/stats", f.issuer))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mut duplicate = Browser {
        cookie: b.cookie.clone(),
        csrf: b.csrf.clone(),
    };
    let (a, c) = tokio::join!(
        send(&f.app, &mut b, "GET", &callback, Value::Null, None, false),
        send(
            &f.app,
            &mut duplicate,
            "GET",
            &callback,
            Value::Null,
            None,
            false
        )
    );
    let mut statuses = [a.0, c.0];
    statuses.sort();
    assert_eq!(statuses, [303, 401]);
    let after: Value = http
        .get(format!("{}/__test/stats", f.issuer))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        after["exchange_count"].as_i64().unwrap(),
        before["exchange_count"].as_i64().unwrap() + 1
    );
}

#[tokio::test]
async fn store_collision_absolute_expiry_and_failures() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let store = Store { pool, now: || 100 };
    store.migrate().await.unwrap();
    let mut first = Record {
        id: Id::default(),
        data: Default::default(),
        expiry_date: time::OffsetDateTime::from_unix_timestamp(200).unwrap(),
    };
    store.create(&mut first).await.unwrap();
    let mut second = first.clone();
    store.create(&mut second).await.unwrap();
    assert_ne!(first.id, second.id);
    first.expiry_date = time::OffsetDateTime::from_unix_timestamp(500).unwrap();
    store.save(&first).await.unwrap();
    assert_eq!(
        store
            .load(&first.id)
            .await
            .unwrap()
            .unwrap()
            .expiry_date
            .unix_timestamp(),
        200
    );
    first.expiry_date = time::OffsetDateTime::from_unix_timestamp(100).unwrap();
    store.save(&first).await.unwrap();
    assert!(store.load(&first.id).await.unwrap().is_none());
    assert!(store.save(&first).await.is_err());
    store.cleanup().await.unwrap();
    store.pool.close().await;
    assert!(store.load(&second.id).await.is_err());
}

#[tokio::test]
async fn logout_wins_against_callback_waiting_on_provider() {
    let f = Fixture::new().await;
    let mut browser = Browser::default();
    login(&f, &mut browser, "alice").await;
    let callback = authorize(&start(&f, &mut browser).await, "bob").await;
    let http = reqwest::Client::new();
    http.post(format!("{}/__test/hold", f.issuer))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let app = f.app.clone();
    let mut pending_browser = Browser {
        cookie: browser.cookie.clone(),
        csrf: browser.csrf.clone(),
    };
    let pending = tokio::spawn(async move {
        send(
            &app,
            &mut pending_browser,
            "GET",
            &callback,
            Value::Null,
            None,
            false,
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let stats: Value = http
                .get(format!("{}/__test/stats", f.issuer))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            if stats["waiting"] == true {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    evidence::record("provider_waiting", 1);
    let logout_status = send(
        &f.app,
        &mut browser,
        "POST",
        "/api/auth/logout",
        Value::Null,
        Some("http://127.0.0.1:5173"),
        true,
    )
    .await
    .0;
    evidence::record("logout_status", i64::from(logout_status));
    assert_eq!(logout_status, 204);
    http.post(format!("{}/__test/release", f.issuer))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let callback_status = pending.await.unwrap().0;
    evidence::record("callback_status", i64::from(callback_status));
    assert_eq!(callback_status, 401);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM iris_sessions")
        .fetch_one(&f.store.pool)
        .await
        .unwrap();
    evidence::record("remaining_sessions", count);
    assert_eq!(count, 0);
}

#[tokio::test]
async fn unmapped_identity_and_expired_attempt_are_rejected() {
    let f = Fixture::new().await;
    sqlx::query("DELETE FROM iris_external_identities WHERE subject='bob'")
        .execute(&f.store.pool)
        .await
        .unwrap();
    let mut browser = Browser::default();
    bootstrap(&f, &mut browser).await;
    let callback = authorize(&start(&f, &mut browser).await, "bob").await;
    assert_eq!(
        send(
            &f.app,
            &mut browser,
            "GET",
            &callback,
            Value::Null,
            None,
            false
        )
        .await
        .0,
        401
    );
    assert!(bootstrap(&f, &mut browser).await["user_id"].is_null());
    let callback = authorize(&start(&f, &mut browser).await, "alice").await;
    sqlx::query("UPDATE iris_login_attempts SET expires_at=0")
        .execute(&f.store.pool)
        .await
        .unwrap();
    assert_eq!(
        send(
            &f.app,
            &mut browser,
            "GET",
            &callback,
            Value::Null,
            None,
            false
        )
        .await
        .0,
        401
    );
    assert!(bootstrap(&f, &mut browser).await["user_id"].is_null());
    f.store.pool.close().await;
    assert_eq!(
        send(
            &f.app,
            &mut browser,
            "GET",
            "/api/auth/session",
            Value::Null,
            None,
            false
        )
        .await
        .0,
        500
    );
}

#[tokio::test]
async fn https_cookie_and_invalid_origin_configuration() {
    let f = Fixture::new().await;
    for origin in [
        "http://example.com",
        "https://example.com/path",
        "https://example.com/",
    ] {
        assert!(
            Auth::discover(
                f.store.clone(),
                origin.into(),
                f.issuer.clone(),
                "iris-local".into(),
                None
            )
            .await
            .is_err()
        );
    }
    let auth = Auth::discover(
        f.store.clone(),
        "https://example.com".into(),
        f.issuer.clone(),
        "iris-local".into(),
        None,
    )
    .await
    .unwrap();
    let app = auth.layer(utoipa_router().0).with_state(AppState {
        database: f._dir.path().join("app.db"),
        now: unix_time,
    });
    let (_, headers, _) = send(
        &app,
        &mut Browser::default(),
        "GET",
        "/api/auth/session",
        Value::Null,
        None,
        false,
    )
    .await;
    let cookie = headers["set-cookie"].to_str().unwrap();
    assert!(cookie.starts_with("__Host-iris-session="));
    assert!(
        cookie.contains("Secure")
            && cookie.contains("HttpOnly")
            && cookie.contains("SameSite=Lax")
            && cookie.contains("Path=/")
    );
    assert!(!cookie.contains("Domain="));
}
