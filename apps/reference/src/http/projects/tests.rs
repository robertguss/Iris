//! `projects.list_mine` (`listMyProjects`): checkpoint B's filter-style
//! authorization, pagination, input and HEAD rows. Expectations are written by
//! hand, not derived from the document under test.
use super::*;
use crate::app::connect;
use crate::http::memberships::{
    list_tests::{contacts, raw, read_request},
    tests::{Fixture, body, capture, collect, remove_body, remove_request, request},
};
use axum::{
    extract::Request,
    middleware::{self, Next},
};
use serde_json::{Value, json};
use sqlx::Connection;
use tower::ServiceExt;

const PATH: &str = "/api/projects";

fn projects(query: &str) -> String {
    if query.is_empty() {
        PATH.to_owned()
    } else {
        format!("{PATH}?{query}")
    }
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

fn validate(status: u16, body: &Value) -> bool {
    let doc = serde_json::to_value(list_mine().api).unwrap();
    let mut schema = doc["paths"][PATH]["get"]["responses"][status.to_string()]["content"]
        ["application/json"]["schema"]
        .clone();
    schema["components"] = doc["components"].clone();
    jsonschema::draft202012::new(&schema)
        .unwrap()
        .is_valid(body)
}

fn expect(response: &(u16, Value), status: u16, kind: &str, code: Option<&str>) {
    assert_eq!(response.0, status, "{response:?}");
    assert_eq!(response.1["operation"], "projects.list_mine");
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
        .map(|project| project["project_id"].as_str().unwrap().parse().unwrap())
        .collect()
}

fn next_cursor(response: &(u16, Value)) -> Option<String> {
    response.1["data"]["next_cursor"]
        .as_str()
        .map(str::to_owned)
}

fn project(id: &str, name: &str, role: &str) -> Value {
    json!({"project_id": id, "name": name, "role": role})
}

#[test]
fn list_mine_independent_contract() {
    let doc = serde_json::to_value(list_mine().api).unwrap();
    assert_eq!(
        doc["paths"].as_object().unwrap().keys().collect::<Vec<_>>(),
        [PATH]
    );
    assert_eq!(
        doc["paths"][PATH]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["get"],
        "HEAD is served as GET, not declared"
    );
    let op = &doc["paths"][PATH]["get"];
    assert_eq!(op["operationId"], "listMyProjects");
    assert_eq!(
        op["x-iris"],
        json!({"operation": "projects.list_mine", "schema_version": 1, "prerequisites": {}}),
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
            ("500", "failure", "iris.internal"),
            ("503", "failure", "iris.unavailable"),
        ],
        "rows are filtered, so there is no refusal beyond the shared profile"
    );
    let schemas = &doc["components"]["schemas"];
    assert_eq!(
        schemas["ProjectPage"]["required"],
        json!(["items", "next_cursor"])
    );
    assert_eq!(
        schemas["ProjectSummary"]["required"],
        json!(["project_id", "name", "role"])
    );
    assert_eq!(
        schemas["Role"]["enum"],
        json!(["owner", "editor", "viewer"])
    );
    let page = json!({"schema_version": 1, "operation": "projects.list_mine",
        "request_id": "req_00000000000000000000000000000000", "kind": "success",
        "data": {"items": [project("41", "Launch plan", "owner")], "next_cursor": null}});
    assert!(validate(200, &page));
    for (pointer, value) in [
        ("/data/next_cursor", json!(5)),
        ("/data/items/0/project_id", json!(41)),
        ("/data/items/0/project_id", json!("041")),
        ("/data/items/0/role", json!("admin")),
        ("/operation", json!("memberships.list")),
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
    assert!(op["responses"].get("403").is_none(), "no 403 is declared");
}

async fn as_alice(mut request: Request, next: Next) -> Response {
    request.extensions_mut().insert(Actor(11));
    next.run(request).await
}

#[tokio::test]
async fn list_mine_whole_request() {
    let f = Fixture::new().await;
    contacts(&f).await;
    let alice = f.cookie(Some(11)).await;
    let bob = f.cookie(Some(29)).await;
    let anonymous = f.cookie(None).await;
    let mut fixtures = vec![];
    for (cookie, query, status, kind, code) in [
        (Some(&alice), "", 200, "success", None),
        (Some(&bob), "limit=1", 200, "success", None),
        (
            Some(&anonymous),
            "",
            401,
            "refused",
            Some("http.unauthenticated"),
        ),
        (None, "", 401, "refused", Some("http.unauthenticated")),
        (
            Some(&alice),
            "limit=0",
            400,
            "refused",
            Some("http.invalid_request"),
        ),
        (
            Some(&alice),
            "cursor=c2.41",
            400,
            "refused",
            Some("http.invalid_request"),
        ),
        (
            Some(&alice),
            "sort=name",
            400,
            "refused",
            Some("http.invalid_request"),
        ),
    ] {
        let response = list(&f, cookie.map(String::as_str), &projects(query)).await;
        expect(&response, status, kind, code);
        fixtures.push(response);
    }
    assert_eq!(
        fixtures[0].1["data"],
        json!({"items": [project("41", "Launch plan", "owner")], "next_cursor": null})
    );
    assert_eq!(
        fixtures[1].1["data"],
        json!({"items": [project("41", "Launch plan", "editor")], "next_cursor": "c1.41"})
    );
    // Busy: the operation's own mount with an injected actor and no cookie,
    // so the session layer never needs the database the test holds
    // exclusively. Not an ordinary cookie-authenticated request.
    let mut locker = connect(&f.state.database).await.unwrap();
    let lock = locker.begin_with("BEGIN EXCLUSIVE").await.unwrap();
    let busy = mount_list_mine(
        list_mine().router.layer(middleware::from_fn(as_alice)),
        f.auth.clone(),
    )
    .with_state(f.state.clone());
    let response = collect(busy.oneshot(read_request("GET", None, PATH)).await.unwrap()).await;
    expect(&response, 503, "failure", Some("iris.unavailable"));
    fixtures.push(response);
    lock.rollback().await.unwrap();
    // A stored name the page cannot decode fails the page query.
    sqlx::raw_sql(
        "INSERT INTO projects (id, name) VALUES (44, X'00');
         INSERT INTO memberships VALUES (44, 11, 'owner');",
    )
    .execute(&f.store.pool)
    .await
    .unwrap();
    let response = list(&f, Some(&alice), PATH).await;
    expect(&response, 500, "failure", Some("iris.internal"));
    fixtures.push(response);
    let ids = fixtures
        .iter()
        .map(|(_, v)| v["request_id"].as_str().unwrap())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), fixtures.len());
    capture("listMyProjects", &fixtures);
}

#[tokio::test]
async fn list_mine_authorization() {
    let f = Fixture::new().await;
    sqlx::query("INSERT INTO users VALUES (31, 'Carol Example')")
        .execute(&f.store.pool)
        .await
        .unwrap();
    contacts(&f).await;
    let alice = f.cookie(Some(11)).await;
    let bob = f.cookie(Some(29)).await;
    let carol = f.cookie(Some(31)).await;
    // Each actor sees exactly their own memberships, with their own role, and
    // without a CSRF token.
    for (cookie, items) in [
        (&alice, json!([project("41", "Launch plan", "owner")])),
        (
            &bob,
            json!([
                project("41", "Launch plan", "editor"),
                project("43", "Field notes", "owner")
            ]),
        ),
        (&carol, json!([])),
    ] {
        let response = list(&f, Some(cookie), PATH).await;
        expect(&response, 200, "success", None);
        assert_eq!(
            response.1["data"],
            json!({"items": items, "next_cursor": null})
        );
    }
    let alices = list(&f, Some(&alice), PATH).await;
    assert!(
        !alices.1.to_string().contains("Field notes"),
        "another project's name never appears"
    );
    // Present state: after Alice removes Bob from 41, Bob's next read omits it.
    let removed = collect(
        f.app()
            .oneshot(remove_request(&alice, &remove_body(29)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(removed.0, 200, "{removed:?}");
    let bobs = list(&f, Some(&bob), PATH).await;
    assert_eq!(
        bobs.1["data"],
        json!({"items": [project("43", "Field notes", "owner")], "next_cursor": null})
    );
    // POST with the same session still needs its CSRF token.
    let mut unsafe_request = request(&alice, &body(29, "viewer"));
    unsafe_request.headers_mut().remove("x-iris-csrf");
    let refused = collect(f.app().oneshot(unsafe_request).await.unwrap()).await;
    assert_eq!(
        (refused.0, refused.1["code"].as_str()),
        (403, Some("http.csrf_refused"))
    );
}

#[tokio::test]
async fn list_mine_pagination() {
    let f = Fixture::new().await;
    // Bob belongs to 41 (editor), 43 (owner) and every project from 1001 to
    // 1120 as a viewer, except 1060, which only Alice belongs to. Alice also
    // belongs to 1001, so her traversal yields a cursor inside Bob's range.
    sqlx::raw_sql(
        "WITH RECURSIVE n(id) AS (SELECT 1001 UNION ALL SELECT id + 1 FROM n WHERE id < 1120)
         INSERT INTO projects SELECT id, 'Project ' || id FROM n;
         INSERT INTO memberships SELECT id, 29, 'viewer' FROM projects WHERE id BETWEEN 1001 AND 1120 AND id <> 1060;
         INSERT INTO memberships VALUES (1060, 11, 'owner'), (1001, 11, 'viewer');",
    )
    .execute(&f.store.pool)
    .await
    .unwrap();
    contacts(&f).await;
    let expected = [41, 43]
        .into_iter()
        .chain((1001..=1120).filter(|id| *id != 1060))
        .collect::<Vec<i64>>();
    let alice = f.cookie(Some(11)).await;
    let bob = f.cookie(Some(29)).await;
    for (query, size) in [("", 50), ("limit=1", 1), ("limit=100", 100)] {
        let page = list(&f, Some(&bob), &projects(query)).await;
        expect(&page, 200, "success", None);
        assert_eq!(ids(&page), expected[..size], "{query:?}");
        assert_eq!(
            next_cursor(&page),
            Some(format!("c1.{}", expected[size - 1]))
        );
    }
    // Forward traversal of an unchanged dataset: each project once, in order,
    // with Bob's own role on each.
    let mut seen = vec![];
    let mut cursor: Option<String> = None;
    let mut pages = 0;
    loop {
        let query = match &cursor {
            None => "limit=7".to_owned(),
            Some(c) => format!("limit=7&cursor={c}"),
        };
        let page = list(&f, Some(&bob), &projects(&query)).await;
        expect(&page, 200, "success", None);
        for item in page.1["data"]["items"].as_array().unwrap() {
            let id = item["project_id"].as_str().unwrap().parse::<i64>().unwrap();
            let role = match id {
                41 => "editor",
                43 => "owner",
                _ => "viewer",
            };
            assert_eq!(item["role"], role, "{id}");
            seen.push(id);
        }
        pages += 1;
        cursor = next_cursor(&page);
        if cursor.is_none() {
            break;
        }
        assert!(pages < 100, "traversal must end");
    }
    assert_eq!(seen, expected);
    assert_eq!(pages, 18);
    // Alice's cursor repositions Bob within his own rows. The page spans 1060,
    // Alice's private project, so its absence is authorization, not paging.
    let alices = list(&f, Some(&alice), &projects("limit=2")).await;
    assert_eq!(ids(&alices), [41, 1001]);
    let foreign = next_cursor(&alices).unwrap();
    let page = list(
        &f,
        Some(&bob),
        &projects(&format!("limit=100&cursor={foreign}")),
    )
    .await;
    let after = expected
        .iter()
        .copied()
        .filter(|id| *id > 1001)
        .take(100)
        .collect::<Vec<_>>();
    assert!(after.first() < Some(&1060) && after.last() > Some(&1060));
    assert_eq!(ids(&page), after);
    // Forged positions stay within the same rows.
    let page = list(&f, Some(&bob), &projects("cursor=c1.1119")).await;
    assert_eq!((ids(&page), next_cursor(&page)), (vec![1120], None));
    let page = list(&f, Some(&bob), &projects("cursor=c1.999999")).await;
    assert_eq!((ids(&page), next_cursor(&page)), (vec![], None));
}

#[tokio::test]
async fn list_mine_input_refusals() {
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
        "cursor=c2.41",
        "cursor=c1.041",
        "cursor=c1.0",
        "cursor=c1.abc",
        "cursor=41",
        "cursor=c1.9223372036854775808",
        "sort=name",
        "limit=5&limit=6",
        "cursor=c1.41&cursor=c1.43",
        "limit=5&extra",
    ];
    queries.push(&overlong);
    for query in queries {
        let response = list(&f, Some(&alice), &projects(query)).await;
        assert_eq!(response.0, 400, "{query}: {response:?}");
        expect(&response, 400, "refused", Some("http.invalid_request"));
    }
    // The session is checked before any input, whether the query fails to
    // extract or a value fails to parse.
    for query in ["sort=name", "limit=0"] {
        expect(
            &list(&f, None, &projects(query)).await,
            401,
            "refused",
            Some("http.unauthenticated"),
        );
    }
}

#[tokio::test]
async fn list_mine_head_is_get_without_a_body() {
    let f = Fixture::new().await;
    let alice = f.cookie(Some(11)).await;
    for (cookie, uri, status) in [
        (Some(alice.as_str()), projects(""), 200),
        (None, projects(""), 401),
        (Some(alice.as_str()), projects("limit=0"), 400),
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
