//! `memberships.list` (`listProjectMembers`): checkpoint B's authorization,
//! pagination, input and HEAD rows. Expectations are written by hand, not
//! derived from the document under test.
use super::tests::{Fixture, body, capture, collect, remove_body, remove_request, request};
use super::*;
use crate::app::connect;
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, Request as HttpRequest},
    middleware::{self, Next},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::Connection;
use tower::ServiceExt;

const TEMPLATE: &str = "/api/projects/{project_id}/members";

fn members(project: &str, query: &str) -> String {
    let query = if query.is_empty() {
        String::new()
    } else {
        format!("?{query}")
    };
    format!("/api/projects/{project}/members{query}")
}

/// A same-origin browser read carries its cookie and nothing else: no Origin
/// and no CSRF token.
pub(crate) fn read_request(method: &str, cookie: Option<&str>, uri: &str) -> HttpRequest<Body> {
    let mut builder = HttpRequest::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    builder.body(Body::empty()).unwrap()
}

async fn list(f: &Fixture, cookie: Option<&str>, uri: &str) -> (u16, Value) {
    collect(
        f.app()
            .oneshot(read_request("GET", cookie, uri))
            .await
            .unwrap(),
    )
    .await
}

pub(crate) async fn raw(
    f: &Fixture,
    method: &str,
    cookie: Option<&str>,
    uri: &str,
) -> (u16, HeaderMap, Vec<u8>) {
    let response = f
        .app()
        .oneshot(read_request(method, cookie, uri))
        .await
        .unwrap();
    let (parts, body) = response.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes().to_vec();
    (parts.status.as_u16(), parts.headers, bytes)
}

fn validate(status: u16, body: &Value) -> bool {
    let doc = serde_json::to_value(list_members().api).unwrap();
    let mut schema = doc["paths"][TEMPLATE]["get"]["responses"][status.to_string()]["content"]
        ["application/json"]["schema"]
        .clone();
    schema["components"] = doc["components"].clone();
    jsonschema::draft202012::new(&schema)
        .unwrap()
        .is_valid(body)
}

fn expect(response: &(u16, Value), status: u16, kind: &str, code: Option<&str>) {
    assert_eq!(response.0, status, "{response:?}");
    assert_eq!(response.1["operation"], "memberships.list");
    assert_eq!(response.1["kind"], kind);
    assert_eq!(response.1.get("code").and_then(Value::as_str), code);
    assert!(
        validate(status, &response.1),
        "schema rejected {response:?}"
    );
}

fn ids(response: &(u16, Value)) -> Vec<i64> {
    response.1["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|member| member["user_id"].as_str().unwrap().parse().unwrap())
        .collect()
}

fn next_cursor(response: &(u16, Value)) -> Option<String> {
    response.1["data"]["next_cursor"]
        .as_str()
        .map(str::to_owned)
}

/// Every user gets a canary address; `collect` fails on any body containing
/// "canary", so a disclosed contact would show.
pub(crate) async fn contacts(f: &Fixture) {
    sqlx::query(
        "INSERT INTO user_contacts (user_id, email)
         SELECT id, 'user-' || id || '-email-canary@example.test' FROM users",
    )
    .execute(&f.store.pool)
    .await
    .unwrap();
}

fn member(id: &str, name: &str, role: &str) -> Value {
    json!({"user_id": id, "display_name": name, "role": role})
}

#[test]
fn list_members_independent_contract() {
    let doc = serde_json::to_value(list_members().api).unwrap();
    assert_eq!(
        doc["paths"].as_object().unwrap().keys().collect::<Vec<_>>(),
        [TEMPLATE]
    );
    assert_eq!(
        doc["paths"][TEMPLATE]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["get"],
        "HEAD is served as GET, not declared"
    );
    let op = &doc["paths"][TEMPLATE]["get"];
    assert_eq!(op["operationId"], "listProjectMembers");
    assert_eq!(
        op["x-iris"],
        json!({"operation": "memberships.list", "schema_version": 1, "prerequisites": {}}),
        "reads declare no recovery capabilities"
    );
    assert!(op.get("requestBody").is_none());
    assert!(op.get("summary").is_none() && op.get("description").is_none());
    assert_eq!(op["security"], json!([{"BrowserSession": []}]));
    let parameters = op["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["name"].clone(),
                p["in"].clone(),
                p["required"].clone(),
                p["schema"].clone(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        parameters,
        [
            (
                json!("project_id"),
                json!("path"),
                json!(true),
                json!({"type": "string", "maxLength": 19, "pattern": "^[1-9][0-9]*$"})
            ),
            (
                json!("limit"),
                json!("query"),
                json!(false),
                json!({"type": "string", "pattern": "^(100|[1-9][0-9]?)$"})
            ),
            (
                json!("cursor"),
                json!("query"),
                json!(false),
                json!({"type": "string", "maxLength": 32, "pattern": "^c1\\.[1-9][0-9]*$"})
            ),
        ]
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
    actual.sort();
    assert_eq!(
        actual,
        [
            ("200", "success", ""),
            ("400", "refused", "http.invalid_request"),
            ("401", "refused", "http.unauthenticated"),
            ("403", "rejected", "memberships.forbidden"),
            ("500", "failure", "iris.internal"),
            ("503", "failure", "iris.unavailable"),
        ],
        "no CSRF refusal for a safe method"
    );
    let schemas = &doc["components"]["schemas"];
    assert_eq!(
        schemas["MemberPage"]["required"],
        json!(["items", "next_cursor"])
    );
    assert_eq!(
        schemas["MemberSummary"]["required"],
        json!(["user_id", "display_name", "role"])
    );
    assert_eq!(
        schemas["Role"]["enum"],
        json!(["owner", "editor", "viewer"])
    );
    let page = json!({"schema_version": 1, "operation": "memberships.list",
        "request_id": "req_00000000000000000000000000000000", "kind": "success",
        "data": {"items": [member("11", "Alice Example", "owner")], "next_cursor": null}});
    assert!(validate(200, &page));
    for (pointer, value) in [
        ("/data/next_cursor", json!(5)),
        ("/data/items/0/user_id", json!(11)),
        ("/data/items/0/user_id", json!("011")),
        ("/data/items/0/role", json!("admin")),
        ("/operation", json!("memberships.change_role")),
        ("/kind", json!("rejected")),
    ] {
        let mut wrong = page.clone();
        *wrong.pointer_mut(pointer).unwrap() = value;
        assert!(!validate(200, &wrong), "{pointer}");
    }
    let mut missing = page.clone();
    missing["data"]
        .as_object_mut()
        .unwrap()
        .remove("next_cursor");
    assert!(!validate(200, &missing), "next_cursor is always present");
}

async fn as_alice(mut request: Request, next: Next) -> Response {
    request.extensions_mut().insert(Actor(11));
    next.run(request).await
}

#[tokio::test]
async fn list_members_whole_request() {
    let f = Fixture::new().await;
    contacts(&f).await;
    let alice = f.cookie(Some(11)).await;
    let bob = f.cookie(Some(29)).await;
    let anonymous = f.cookie(None).await;
    let mut fixtures = vec![];
    for (cookie, uri, status, kind, code) in [
        (Some(&alice), members("41", ""), 200, "success", None),
        (Some(&bob), members("41", "limit=1"), 200, "success", None),
        (
            Some(&alice),
            members("43", ""),
            403,
            "rejected",
            Some("memberships.forbidden"),
        ),
        (
            Some(&alice),
            members("999", ""),
            403,
            "rejected",
            Some("memberships.forbidden"),
        ),
        (
            Some(&anonymous),
            members("41", ""),
            401,
            "refused",
            Some("http.unauthenticated"),
        ),
        (
            None,
            members("41", ""),
            401,
            "refused",
            Some("http.unauthenticated"),
        ),
        (
            Some(&alice),
            members("41", "limit=0"),
            400,
            "refused",
            Some("http.invalid_request"),
        ),
        (
            Some(&alice),
            members("41", "cursor=c2.11"),
            400,
            "refused",
            Some("http.invalid_request"),
        ),
        (
            Some(&alice),
            members("41", "sort=name"),
            400,
            "refused",
            Some("http.invalid_request"),
        ),
        (
            Some(&alice),
            members("01", ""),
            400,
            "refused",
            Some("http.invalid_request"),
        ),
    ] {
        let response = list(&f, cookie.map(String::as_str), &uri).await;
        expect(&response, status, kind, code);
        fixtures.push(response);
    }
    assert_eq!(
        fixtures[0].1["data"],
        json!({"items": [member("11", "Alice Example", "owner"), member("29", "Bob Example", "editor")],
            "next_cursor": null})
    );
    assert_eq!(
        fixtures[1].1["data"],
        json!({"items": [member("11", "Alice Example", "owner")], "next_cursor": "c1.11"})
    );
    // Busy: the operation's own mount with an injected actor and no cookie,
    // so the session layer never needs the database the test holds
    // exclusively. Not an ordinary cookie-authenticated request.
    let mut locker = connect(&f.state.database).await.unwrap();
    let lock = locker.begin_with("BEGIN EXCLUSIVE").await.unwrap();
    let busy = mount_list_members(
        list_members().router.layer(middleware::from_fn(as_alice)),
        f.auth.clone(),
    )
    .with_state(f.state.clone());
    let response = collect(
        busy.oneshot(read_request("GET", None, &members("41", "")))
            .await
            .unwrap(),
    )
    .await;
    expect(&response, 503, "failure", Some("iris.unavailable"));
    fixtures.push(response);
    lock.rollback().await.unwrap();
    // A stored value the page cannot decode fails the page query, not the
    // session check, which reads only user IDs.
    sqlx::raw_sql(
        "INSERT INTO users (id, display_name) VALUES (30, X'00');
         INSERT INTO memberships VALUES (41, 30, 'viewer');",
    )
    .execute(&f.store.pool)
    .await
    .unwrap();
    let response = list(&f, Some(&alice), &members("41", "")).await;
    expect(&response, 500, "failure", Some("iris.internal"));
    fixtures.push(response);
    let ids = fixtures
        .iter()
        .map(|(_, v)| v["request_id"].as_str().unwrap())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), fixtures.len());
    capture("listProjectMembers", &fixtures);
}

#[tokio::test]
async fn list_members_authorization() {
    let f = Fixture::new().await;
    sqlx::raw_sql(
        "INSERT INTO users VALUES (31, 'Carol Example');
         INSERT INTO memberships VALUES (41, 31, 'viewer');",
    )
    .execute(&f.store.pool)
    .await
    .unwrap();
    contacts(&f).await;
    let alice = f.cookie(Some(11)).await;
    let bob = f.cookie(Some(29)).await;
    let carol = f.cookie(Some(31)).await;
    // Any member, in any role, reads the same members without a CSRF token.
    for cookie in [&alice, &bob, &carol] {
        let response = list(&f, Some(cookie), &members("41", "")).await;
        expect(&response, 200, "success", None);
        assert_eq!(
            response.1["data"],
            json!({"items": [member("11", "Alice Example", "owner"), member("29", "Bob Example", "editor"),
                member("31", "Carol Example", "viewer")], "next_cursor": null})
        );
    }
    // An unknown project and a non-member's project: the same response apart
    // from request_id, headers included.
    let non_member = raw(&f, "GET", Some(&alice), &members("43", "")).await;
    let unknown = raw(&f, "GET", Some(&alice), &members("999", "")).await;
    assert_eq!(non_member.0, 403);
    assert_eq!((non_member.0, &non_member.1), (unknown.0, &unknown.1));
    let without_id = |bytes: &[u8]| {
        let mut value: Value = serde_json::from_slice(bytes).unwrap();
        assert!(
            value
                .as_object_mut()
                .unwrap()
                .remove("request_id")
                .is_some()
        );
        value
    };
    assert_eq!(without_id(&non_member.2), without_id(&unknown.2));
    expect(
        &list(&f, Some(&alice), &members("999", "")).await,
        403,
        "rejected",
        Some("memberships.forbidden"),
    );
    // Permitted disclosures stay distinct from that refusal.
    expect(
        &list(&f, Some(&alice), &members("abc", "")).await,
        400,
        "refused",
        Some("http.invalid_request"),
    );
    expect(
        &list(&f, None, &members("43", "")).await,
        401,
        "refused",
        Some("http.unauthenticated"),
    );
    // Visibility is re-evaluated for every page.
    let first = list(&f, Some(&bob), &members("41", "limit=1")).await;
    assert_eq!(ids(&first), [11]);
    let cursor = next_cursor(&first).unwrap();
    let removed = collect(
        f.app()
            .oneshot(remove_request(&alice, &remove_body(29)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(removed.0, 200, "{removed:?}");
    expect(
        &list(
            &f,
            Some(&bob),
            &members("41", &format!("limit=1&cursor={cursor}")),
        )
        .await,
        403,
        "rejected",
        Some("memberships.forbidden"),
    );
    // POST with the same session still needs its CSRF token, and a mutation
    // after these reads commits.
    let mut unsafe_request = request(&alice, &body(31, "editor"));
    unsafe_request.headers_mut().remove("x-iris-csrf");
    let refused = collect(f.app().oneshot(unsafe_request).await.unwrap()).await;
    assert_eq!(
        (refused.0, refused.1["code"].as_str()),
        (403, Some("http.csrf_refused"))
    );
    let changed = collect(
        f.app()
            .oneshot(request(&alice, &body(31, "editor")))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(changed.0, 200, "{changed:?}");
    let page = list(&f, Some(&carol), &members("41", "")).await;
    assert_eq!(
        page.1["data"]["items"][1],
        member("31", "Carol Example", "editor")
    );
}

#[tokio::test]
async fn list_members_pagination() {
    let f = Fixture::new().await;
    sqlx::raw_sql(
        "WITH RECURSIVE n(id) AS (SELECT 1001 UNION ALL SELECT id + 1 FROM n WHERE id < 1120)
         INSERT INTO users SELECT id, 'Member ' || id FROM n;
         INSERT INTO memberships SELECT 41, id, 'viewer' FROM users WHERE id BETWEEN 1001 AND 1120;
         INSERT INTO users VALUES (1500, 'Outside Example');
         INSERT INTO memberships VALUES (43, 1500, 'viewer');",
    )
    .execute(&f.store.pool)
    .await
    .unwrap();
    contacts(&f).await;
    let expected = [11, 29]
        .into_iter()
        .chain(1001..=1120)
        .collect::<Vec<i64>>();
    let alice = f.cookie(Some(11)).await;
    let bob = f.cookie(Some(29)).await;
    for (query, size) in [("", 50), ("limit=1", 1), ("limit=100", 100)] {
        let page = list(&f, Some(&alice), &members("41", query)).await;
        expect(&page, 200, "success", None);
        assert_eq!(ids(&page), expected[..size], "{query:?}");
        assert_eq!(
            next_cursor(&page),
            Some(format!("c1.{}", expected[size - 1]))
        );
    }
    // Forward traversal of an unchanged dataset: every member once, in order.
    let mut seen = vec![];
    let mut cursor: Option<String> = None;
    let mut pages = 0;
    loop {
        let query = match &cursor {
            None => "limit=7".to_owned(),
            Some(c) => format!("limit=7&cursor={c}"),
        };
        let page = list(&f, Some(&alice), &members("41", &query)).await;
        expect(&page, 200, "success", None);
        seen.extend(ids(&page));
        pages += 1;
        cursor = next_cursor(&page);
        if cursor.is_none() {
            break;
        }
        assert!(pages < 100, "traversal must end");
    }
    assert_eq!(seen, expected);
    assert_eq!(pages, 18);
    // A cursor from another project's traversal repositions within the
    // caller's rows; it never reaches rows the caller cannot see.
    let bobs = list(&f, Some(&bob), &members("43", "limit=1")).await;
    assert_eq!(ids(&bobs), [29]);
    let foreign = next_cursor(&bobs).unwrap();
    let page = list(
        &f,
        Some(&alice),
        &members("41", &format!("limit=100&cursor={foreign}")),
    )
    .await;
    assert_eq!(ids(&page), expected[2..102]);
    expect(
        &list(
            &f,
            Some(&alice),
            &members("43", &format!("cursor={foreign}")),
        )
        .await,
        403,
        "rejected",
        Some("memberships.forbidden"),
    );
    // Forged positions stay within the same rows.
    let page = list(&f, Some(&alice), &members("41", "cursor=c1.1119")).await;
    assert_eq!((ids(&page), next_cursor(&page)), (vec![1120], None));
    let page = list(&f, Some(&alice), &members("41", "cursor=c1.999999")).await;
    assert_eq!((ids(&page), next_cursor(&page)), (vec![], None));
}

#[tokio::test]
async fn list_members_input_refusals() {
    let f = Fixture::new().await;
    let alice = f.cookie(Some(11)).await;
    let overlong = format!("cursor=c1.{}", "1".repeat(30));
    let mut queries = vec![
        "limit=0",
        "limit=101",
        "limit=abc",
        "limit=05",
        "limit=-1",
        "limit=",
        "limit=%2B5",
        "limit=1.5",
        "limit=4294967297",
        "cursor=",
        "cursor=c2.11",
        "cursor=c1.011",
        "cursor=c1.0",
        "cursor=c1.abc",
        "cursor=11",
        "cursor=c1.9223372036854775808",
        "sort=name",
        "limit=5&limit=6",
        "cursor=c1.11&cursor=c1.29",
        "limit=5&extra",
    ];
    queries.push(&overlong);
    for query in queries {
        let response = list(&f, Some(&alice), &members("41", query)).await;
        assert_eq!(response.0, 400, "{query}: {response:?}");
        expect(&response, 400, "refused", Some("http.invalid_request"));
    }
    for project in ["0", "01", "-1", "abc", "9223372036854775808"] {
        let response = list(&f, Some(&alice), &members(project, "")).await;
        assert_eq!(response.0, 400, "{project}: {response:?}");
        expect(&response, 400, "refused", Some("http.invalid_request"));
    }
    // The session is checked before any input, whether the query fails to
    // extract or a value fails to parse.
    for uri in [members("41", "sort=name"), members("abc", "limit=0")] {
        expect(
            &list(&f, None, &uri).await,
            401,
            "refused",
            Some("http.unauthenticated"),
        );
    }
}

#[tokio::test]
async fn list_members_head_is_get_without_a_body() {
    let f = Fixture::new().await;
    let alice = f.cookie(Some(11)).await;
    for (cookie, uri, status) in [
        (Some(alice.as_str()), members("41", ""), 200),
        (None, members("41", ""), 401),
        (Some(alice.as_str()), members("43", ""), 403),
        (Some(alice.as_str()), members("41", "limit=0"), 400),
    ] {
        let get = raw(&f, "GET", cookie, &uri).await;
        let head = raw(&f, "HEAD", cookie, &uri).await;
        assert_eq!(get.0, status, "{uri}");
        assert_eq!((head.0, &head.1), (get.0, &get.1), "{uri}");
        assert_eq!(head.1["content-type"], "application/json");
        assert_eq!(head.1["cache-control"], "no-store");
        assert!(head.2.is_empty(), "{uri}: HEAD carries no body");
        assert!(!get.2.is_empty());
    }
}
