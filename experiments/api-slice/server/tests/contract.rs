use axum::{Router, body::Body, http::Request};
use http_body_util::BodyExt;
use iris_api_spike::{ACCEPT_PATH, AppState, ISSUE_PATH, aide_router, utoipa_router};
use serde_json::{Value, json};
use tower::ServiceExt;

fn candidates() -> Vec<(&'static str, Router<AppState>, Value)> {
    let (u, us) = utoipa_router();
    let (a, as_) = aide_router();
    vec![
        ("utoipa", u, serde_json::to_value(us).unwrap()),
        ("aide", a, serde_json::to_value(as_).unwrap()),
    ]
}

fn validator(api: &Value, schema: &Value) -> jsonschema::Validator {
    let mut schema = schema.clone();
    schema["components"] = api["components"].clone();
    jsonschema::validator_for(&schema).unwrap()
}

async fn request(
    router: Router,
    identity: Option<&str>,
    body: &str,
    status: u16,
    expected: Value,
    api: &Value,
) {
    request_at(router, ACCEPT_PATH, identity, body, status, expected, api).await;
}

async fn request_at(
    router: Router,
    path: &str,
    identity: Option<&str>,
    body: &str,
    status: u16,
    expected: Value,
    api: &Value,
) -> Value {
    let mut builder = Request::post(path).header("content-type", "application/json");
    if let Some(identity) = identity {
        builder = builder.header("x-iris-dev-user", identity);
    }
    let response = router
        .oneshot(builder.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), status);
    assert_eq!(response.headers()["content-type"], "application/json");
    if status == 201 {
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let actual: Value = serde_json::from_slice(&bytes).unwrap();
    for (key, value) in expected.as_object().unwrap() {
        assert_eq!(&actual[key], value, "{actual}");
    }
    let schema = &api["paths"][path]["post"]["responses"][status.to_string()]["content"]["application/json"]
        ["schema"];
    assert!(!schema.is_null(), "undocumented response {status}");
    validator(api, schema).validate(&actual).unwrap();
    actual
}

#[test]
fn exported_contracts_and_constraints_match() {
    for (name, _, api) in candidates() {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../contracts/{name}.json"));
        let committed: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
        assert_eq!(api, committed, "regenerate {name} contract");
        let operation = &api["paths"][ACCEPT_PATH]["post"];
        assert_eq!(operation["operationId"], "acceptInvitation");
        assert_eq!(operation["security"], json!([{"BrowserSession": []}]));
        assert_eq!(
            api["components"]["securitySchemes"]["BrowserSession"]["name"],
            "__Host-iris-session"
        );
        let statuses: Vec<_> = operation["responses"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            statuses,
            ["200", "400", "401", "403", "404", "409", "500", "503"]
        );
        let input = validator(
            &api,
            &operation["requestBody"]["content"]["application/json"]["schema"],
        );
        assert!(input.is_valid(&json!({"token":"x"})));
        assert!(input.is_valid(&json!({"token":"😀".repeat(256)})));
        for invalid in [
            json!({}),
            json!({"token":""}),
            json!({"token":"x".repeat(257)}),
            json!({"token":"x", "user_id":29}),
        ] {
            assert!(!input.is_valid(&invalid), "{name} accepted {invalid}");
        }
        let success = validator(
            &api,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        );
        assert!(!success.is_valid(&json!({"project_id":7,"user_id":"11"})));
        assert!(!success.is_valid(&json!({"project_id":"7"})));
        let error = validator(
            &api,
            &operation["responses"]["409"]["content"]["application/json"]["schema"],
        );
        assert!(!error.is_valid(&json!({"code":"invented_error","message":"no"})));
    }
}

#[tokio::test]
async fn ordinary_build_rejects_development_identity() {
    for (_, router, api) in candidates() {
        let app = router.with_state(AppState {
            database: "must-not-open.db".into(),
            now: || 100,
        });
        request(
            app.clone(),
            Some("11"),
            r#"{"token":"iris-valid"}"#,
            401,
            json!({"code":"unauthorized"}),
            &api,
        )
        .await;
        request_at(
            app,
            ISSUE_PATH,
            Some("11"),
            r#"{"project_id":"41","recipient_id":"29"}"#,
            401,
            json!({"code":"unauthorized"}),
            &api,
        )
        .await;
    }
}

#[cfg(feature = "dev-identity")]
mod demo {
    use super::*;
    use iris_api_spike::seed_demo;
    use iris_sqlite_spike::{connect, migrate};

    async fn fixture(
        router: Router<AppState>,
    ) -> (tempfile::TempDir, Router, sqlx::SqliteConnection) {
        let router = iris_api_spike::development_identity(router);
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("app.db");
        let mut conn = connect(&database).await.unwrap();
        migrate(&mut conn).await.unwrap();
        seed_demo(&mut conn, 100).await.unwrap();
        (
            dir,
            router.with_state(AppState {
                database,
                now: || 100,
            }),
            conn,
        )
    }

    #[tokio::test]
    async fn member_workflow_and_failures_match_both_contracts() {
        for (_, router, api) in candidates() {
            let (_dir, app, mut conn) = fixture(router).await;
            sqlx::raw_sql("INSERT INTO memberships VALUES(41,29,'editor')")
                .execute(&mut conn)
                .await
                .unwrap();
            for path in ["/api/memberships/role", "/api/memberships/remove"] {
                let mut body = json!({"project_id":"41","user_id":"11"});
                if path.ends_with("role") {
                    body["role"] = json!("viewer");
                }
                for (actor, status, code) in [
                    (None, 401, "unauthorized"),
                    (Some("29"), 403, "forbidden"),
                    (Some("11"), 409, "last_owner"),
                ] {
                    request_at(
                        app.clone(),
                        path,
                        actor,
                        &body.to_string(),
                        status,
                        json!({"code":code}),
                        &api,
                    )
                    .await;
                }
                for (project, user, status, code) in [
                    ("999", "999", 403, "forbidden"),
                    ("43", "11", 403, "forbidden"),
                    ("41", "999", 404, "member_not_found"),
                    ("041", "11", 400, "invalid_request"),
                    ("41", "9223372036854775808", 400, "invalid_request"),
                ] {
                    let mut invalid = body.clone();
                    invalid["project_id"] = json!(project);
                    invalid["user_id"] = json!(user);
                    request_at(
                        app.clone(),
                        path,
                        Some("11"),
                        &invalid.to_string(),
                        status,
                        json!({"code":code}),
                        &api,
                    )
                    .await;
                }
                for invalid in [
                    json!({}),
                    json!({"project_id":"41","user_id":"29","role":"admin"}),
                    json!({"project_id":"41","user_id":"29","actor_id":"11"}),
                ] {
                    request_at(
                        app.clone(),
                        path,
                        Some("11"),
                        &invalid.to_string(),
                        400,
                        json!({"code":"invalid_request"}),
                        &api,
                    )
                    .await;
                }
                body["user_id"] = json!("29");
                sqlx::raw_sql("BEGIN IMMEDIATE")
                    .execute(&mut conn)
                    .await
                    .unwrap();
                request_at(
                    app.clone(),
                    path,
                    Some("11"),
                    &body.to_string(),
                    503,
                    json!({"code":"unavailable"}),
                    &api,
                )
                .await;
                sqlx::raw_sql("ROLLBACK").execute(&mut conn).await.unwrap();
                let trigger = if path.ends_with("role") {
                    "CREATE TRIGGER fail_member AFTER UPDATE ON memberships BEGIN SELECT RAISE(ABORT, 'private detail'); END"
                } else {
                    "CREATE TRIGGER fail_member AFTER DELETE ON memberships BEGIN SELECT RAISE(ABORT, 'private detail'); END"
                };
                // AFTER failure proves the attempted mutation is rolled back.
                sqlx::raw_sql(trigger).execute(&mut conn).await.unwrap();
                request_at(
                    app.clone(),
                    path,
                    Some("11"),
                    &body.to_string(),
                    500,
                    json!({"code":"internal","message":"The request could not be completed."}),
                    &api,
                )
                .await;
                sqlx::raw_sql("DROP TRIGGER fail_member")
                    .execute(&mut conn)
                    .await
                    .unwrap();
                let role: String = sqlx::query_scalar(
                    "SELECT role FROM memberships WHERE project_id=41 AND user_id=29",
                )
                .fetch_one(&mut conn)
                .await
                .unwrap();
                assert_eq!(role, "editor");
            }
            let role_path = "/api/memberships/role";
            for role in ["viewer", "editor", "owner", "owner"] {
                request_at(
                    app.clone(),
                    role_path,
                    Some("11"),
                    &json!({"project_id":"41","user_id":"29","role":role}).to_string(),
                    200,
                    json!({"project_id":"41","user_id":"29","role":role}),
                    &api,
                )
                .await;
                let actual: String = sqlx::query_scalar(
                    "SELECT role FROM memberships WHERE project_id=41 AND user_id=29",
                )
                .fetch_one(&mut conn)
                .await
                .unwrap();
                assert_eq!(actual, role);
            }
            request_at(
                app.clone(),
                "/api/memberships/remove",
                Some("11"),
                r#"{"project_id":"41","user_id":"11"}"#,
                200,
                json!({"project_id":"41","user_id":"11","role":null}),
                &api,
            )
            .await;
            request_at(
                app.clone(),
                role_path,
                Some("11"),
                r#"{"project_id":"41","user_id":"29","role":"viewer"}"#,
                403,
                json!({"code":"forbidden"}),
                &api,
            )
            .await;
            request_at(
                app.clone(),
                "/api/memberships/remove",
                Some("29"),
                r#"{"project_id":"41","user_id":"11"}"#,
                404,
                json!({"code":"member_not_found"}),
                &api,
            )
            .await;
            let remaining: Vec<(i64, String)> =
                sqlx::query_as("SELECT user_id,role FROM memberships WHERE project_id=41")
                    .fetch_all(&mut conn)
                    .await
                    .unwrap();
            assert_eq!(remaining, vec![(29, "owner".into())]);
        }
    }

    #[tokio::test]
    async fn issue_authorize_then_accept_through_both_contracts() {
        for (_, router, api) in candidates() {
            let (_dir, app, mut conn) = fixture(router).await;
            let body = r#"{"project_id":"41","recipient_id":"29"}"#;
            for (actor, status, code) in
                [(None, 401, "unauthorized"), (Some("29"), 403, "forbidden")]
            {
                request_at(
                    app.clone(),
                    ISSUE_PATH,
                    actor,
                    body,
                    status,
                    json!({"code":code}),
                    &api,
                )
                .await;
            }
            for (body, status, code) in [
                (
                    r#"{"project_id":"43","recipient_id":"29"}"#,
                    403,
                    "forbidden",
                ),
                (
                    r#"{"project_id":"999","recipient_id":"999"}"#,
                    403,
                    "forbidden",
                ),
                (
                    r#"{"project_id":"41","recipient_id":"999"}"#,
                    404,
                    "recipient_not_found",
                ),
                (
                    r#"{"project_id":"41","recipient_id":"11"}"#,
                    409,
                    "already_member",
                ),
                (
                    r#"{"project_id":"41","recipient_id":"29","token":"chosen"}"#,
                    400,
                    "invalid_request",
                ),
                (
                    r#"{"project_id":"41","recipient_id":"29","actor_id":"11"}"#,
                    400,
                    "invalid_request",
                ),
                (
                    r#"{"project_id":"41","recipient_id":"29","role":"owner"}"#,
                    400,
                    "invalid_request",
                ),
                (
                    r#"{"project_id":"041","recipient_id":"29"}"#,
                    400,
                    "invalid_request",
                ),
                (
                    r#"{"project_id":"9223372036854775808","recipient_id":"29"}"#,
                    400,
                    "invalid_request",
                ),
                (
                    r#"{"project_id":41,"recipient_id":"29"}"#,
                    400,
                    "invalid_request",
                ),
            ] {
                request_at(
                    app.clone(),
                    ISSUE_PATH,
                    Some("11"),
                    body,
                    status,
                    json!({"code":code}),
                    &api,
                )
                .await;
            }
            let issued = request_at(
                app.clone(),
                ISSUE_PATH,
                Some("11"),
                body,
                201,
                json!({"project_id":"41","recipient_id":"29","expires_at":"3700"}),
                &api,
            )
            .await;
            let token = issued["token"].as_str().unwrap();
            assert_eq!(token.len(), 64);
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM memberships WHERE project_id=41 AND user_id=29",
            )
            .fetch_one(&mut conn)
            .await
            .unwrap();
            assert_eq!(count, 0, "Issuing must not grant membership");
            request_at(
                app.clone(),
                ISSUE_PATH,
                Some("11"),
                body,
                409,
                json!({"code":"invitation_pending"}),
                &api,
            )
            .await;
            let accept = json!({"token": token}).to_string();
            request(
                app.clone(),
                Some("11"),
                &accept,
                404,
                json!({"code":"not_found"}),
                &api,
            )
            .await;
            request(
                app.clone(),
                Some("29"),
                &accept,
                200,
                json!({"project_id":"41","user_id":"29"}),
                &api,
            )
            .await;
            let role: String = sqlx::query_scalar(
                "SELECT role FROM memberships WHERE project_id=41 AND user_id=29",
            )
            .fetch_one(&mut conn)
            .await
            .unwrap();
            assert_eq!(role, "editor");
            request_at(
                app.clone(),
                ISSUE_PATH,
                Some("11"),
                body,
                409,
                json!({"code":"already_member"}),
                &api,
            )
            .await;
            request_at(
                app,
                ISSUE_PATH,
                Some("29"),
                body,
                403,
                json!({"code":"forbidden"}),
                &api,
            )
            .await;
        }
    }

    #[tokio::test]
    async fn issue_database_failures_match_both_contracts() {
        for (_, router, api) in candidates() {
            let (_dir, app, mut conn) = fixture(router).await;
            let body = r#"{"project_id":"41","recipient_id":"29"}"#;
            sqlx::raw_sql("BEGIN IMMEDIATE")
                .execute(&mut conn)
                .await
                .unwrap();
            request_at(
                app.clone(),
                ISSUE_PATH,
                Some("11"),
                body,
                503,
                json!({"code":"unavailable"}),
                &api,
            )
            .await;
            sqlx::raw_sql("ROLLBACK; CREATE TRIGGER fail_issue BEFORE INSERT ON invitations BEGIN SELECT RAISE(ABORT, 'private database detail'); END;")
                .execute(&mut conn).await.unwrap();
            request_at(
                app.clone(),
                ISSUE_PATH,
                Some("11"),
                body,
                500,
                json!({"code":"internal","message":"The request could not be completed."}),
                &api,
            )
            .await;
            let count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM invitations WHERE project_id=41")
                    .fetch_one(&mut conn)
                    .await
                    .unwrap();
            assert_eq!(count, 0);
            sqlx::raw_sql("DROP TRIGGER fail_issue")
                .execute(&mut conn)
                .await
                .unwrap();
            request_at(
                app,
                ISSUE_PATH,
                Some("11"),
                body,
                201,
                json!({"project_id":"41"}),
                &api,
            )
            .await;
        }
    }

    #[tokio::test]
    async fn actual_success_and_domain_errors_match_both_contracts() {
        for (_, router, api) in candidates() {
            let (_dir, app, mut conn) = fixture(router).await;
            request(
                app.clone(),
                None,
                r#"{"token":"iris-valid"}"#,
                401,
                json!({"code":"unauthorized"}),
                &api,
            )
            .await;
            request(
                app.clone(),
                Some("999"),
                r#"{"token":"iris-valid"}"#,
                401,
                json!({"code":"unauthorized"}),
                &api,
            )
            .await;
            request(
                app.clone(),
                Some("29"),
                r#"{"token":"iris-valid"}"#,
                404,
                json!({"code":"not_found"}),
                &api,
            )
            .await;
            request(
                app.clone(),
                Some("11"),
                r#"{"token":"unknown"}"#,
                404,
                json!({"code":"not_found"}),
                &api,
            )
            .await;
            request(
                app.clone(),
                Some("11"),
                r#"{"token":"iris-expired"}"#,
                409,
                json!({"code":"expired"}),
                &api,
            )
            .await;
            request(
                app.clone(),
                Some("11"),
                r#"{"token":"iris-valid"}"#,
                200,
                json!({"project_id":"7","user_id":"11"}),
                &api,
            )
            .await;
            request(
                app.clone(),
                Some("11"),
                r#"{"token":"iris-valid"}"#,
                409,
                json!({"code":"already_accepted"}),
                &api,
            )
            .await;
            request(
                app.clone(),
                Some("29"),
                r#"{"token":"iris-bob"}"#,
                200,
                json!({"project_id":"19","user_id":"29"}),
                &api,
            )
            .await;
            let count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM memberships WHERE project_id IN (7,19)")
                    .fetch_one(&mut conn)
                    .await
                    .unwrap();
            assert_eq!(count, 2);
        }
    }

    #[tokio::test]
    async fn malformed_and_spoofed_bodies_are_documented_errors() {
        for (_, router, api) in candidates() {
            let (_dir, app, mut conn) = fixture(router).await;
            for body in [
                "{",
                "{}",
                r#"{"token":3}"#,
                r#"{"token":""}"#,
                r#"{"token":"iris-valid","user_id":29}"#,
            ] {
                request(
                    app.clone(),
                    Some("11"),
                    body,
                    400,
                    json!({"code":"invalid_request"}),
                    &api,
                )
                .await;
            }
            let too_long = json!({"token":"x".repeat(257)}).to_string();
            request(
                app.clone(),
                Some("11"),
                &too_long,
                400,
                json!({"code":"invalid_request"}),
                &api,
            )
            .await;
            let boundary = json!({"token":"😀".repeat(256)}).to_string();
            request(
                app.clone(),
                Some("11"),
                &boundary,
                404,
                json!({"code":"not_found"}),
                &api,
            )
            .await;
            let count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM memberships WHERE project_id IN (7,19)")
                    .fetch_one(&mut conn)
                    .await
                    .unwrap();
            assert_eq!(count, 0);
        }
    }

    #[tokio::test]
    async fn database_failures_are_sanitized_and_documented() {
        for (_, router, api) in candidates() {
            let (_dir, app, mut conn) = fixture(router).await;
            sqlx::raw_sql("BEGIN IMMEDIATE")
                .execute(&mut conn)
                .await
                .unwrap();
            request(
                app.clone(),
                Some("11"),
                r#"{"token":"iris-valid"}"#,
                503,
                json!({"code":"unavailable"}),
                &api,
            )
            .await;
            sqlx::raw_sql("ROLLBACK; DROP TABLE memberships")
                .execute(&mut conn)
                .await
                .unwrap();
            request(
                app.clone(),
                Some("11"),
                r#"{"token":"iris-valid"}"#,
                500,
                json!({"code":"internal","message":"The request could not be completed."}),
                &api,
            )
            .await;
            let consumed: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM invitations WHERE accepted_at IS NOT NULL",
            )
            .fetch_one(&mut conn)
            .await
            .unwrap();
            assert_eq!(consumed, 0);
        }
    }
}
