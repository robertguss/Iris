use axum::{Router, body::Body, http::Request};
use http_body_util::BodyExt;
use iris_api_spike::{ACCEPT_PATH, AppState, aide_router, utoipa_router};
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
    let mut builder = Request::post(ACCEPT_PATH).header("content-type", "application/json");
    if let Some(identity) = identity {
        builder = builder.header("x-iris-dev-user", identity);
    }
    let response = router
        .oneshot(builder.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), status);
    assert_eq!(response.headers()["content-type"], "application/json");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let actual: Value = serde_json::from_slice(&bytes).unwrap();
    for (key, value) in expected.as_object().unwrap() {
        assert_eq!(&actual[key], value, "{actual}");
    }
    let schema = &api["paths"][ACCEPT_PATH]["post"]["responses"][status.to_string()]["content"]["application/json"]
        ["schema"];
    assert!(!schema.is_null(), "undocumented response {status}");
    validator(api, schema).validate(&actual).unwrap();
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
        assert_eq!(operation["security"], json!([{"DevIdentity": []}]));
        assert_eq!(
            api["components"]["securitySchemes"]["DevIdentity"]["name"],
            "x-iris-dev-user"
        );
        let statuses: Vec<_> = operation["responses"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(statuses, ["200", "400", "401", "404", "409", "500", "503"]);
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

#[cfg(not(feature = "dev-identity"))]
#[tokio::test]
async fn ordinary_build_rejects_development_identity() {
    for (_, router, api) in candidates() {
        let app = router.with_state(AppState {
            database: "must-not-open.db".into(),
            now: || 100,
        });
        request(
            app,
            Some("11"),
            r#"{"token":"iris-valid"}"#,
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
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memberships")
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
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memberships")
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
