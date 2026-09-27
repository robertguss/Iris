//! Assembly checks on synthetic documents; source-copy omission probes are separate.
use super::*;
use axum::http::HeaderMap;

fn document(paths: Value, schemas: Value) -> OpenApi {
    serde_json::from_value(json!({
        "openapi": "3.1.0",
        "info": {"title": "probe", "version": "1"},
        "paths": paths,
        "components": {"schemas": schemas},
    }))
    .unwrap()
}

fn operations(declared: &[(&str, &str, Option<&str>)]) -> OpenApi {
    let paths = declared
        .iter()
        .map(|(path, id, name)| {
            let mut operation = json!({"operationId": id, "responses": {}});
            if let Some(name) = name {
                operation["x-iris"] = json!({"operation": name});
            }
            (path.to_string(), json!({"post": operation}))
        })
        .collect::<serde_json::Map<_, _>>();
    document(Value::Object(paths), json!({}))
}

const FORBIDDEN: Mapping = Mapping {
    status: 403,
    kind: "rejected",
    code: Some("probe.forbidden"),
    message: "Not permitted.",
    rule: None,
    prerequisite: None,
};

fn entry(name: &'static str, public_id: &'static str, mapping: Mapping) -> CatalogEntry {
    CatalogEntry {
        name,
        public_id,
        handler: if name == "probe.a" {
            "a_endpoint"
        } else {
            "b_endpoint"
        },
        mappings: vec![mapping],
    }
}

fn two_operations() -> OpenApi {
    operations(&[
        ("/a", "probeA", Some("probe.a")),
        ("/b", "probeB", Some("probe.b")),
    ])
}

#[test]
fn identical_shared_component_merges() {
    let schema = json!({"type": "string", "enum": ["acknowledged"]});
    let mut into = document(json!({"/a": {}}), json!({"Completion": schema}));
    merge_checked(
        &mut into,
        document(json!({"/b": {}}), json!({"Completion": schema})),
    );
    assert_eq!(into.paths.paths.len(), 2);
}

#[test]
#[should_panic(expected = "component conflict: schemas/Completion")]
fn same_named_different_component_fails() {
    let mut into = document(
        json!({"/a": {}}),
        json!({"Completion": {"type": "string", "enum": ["acknowledged"]}}),
    );
    merge_checked(
        &mut into,
        document(
            json!({"/b": {}}),
            json!({"Completion": {"type": "string", "enum": ["done"]}}),
        ),
    );
}

#[test]
#[should_panic(expected = "path conflict: /a")]
fn same_path_fails() {
    let mut into = document(json!({"/a": {}}), json!({}));
    merge_checked(&mut into, document(json!({"/a": {}}), json!({})));
}

#[test]
fn consistent_catalog_passes() {
    check_catalog(
        &two_operations(),
        &[
            entry("probe.a", "probeA", FORBIDDEN),
            entry("probe.b", "probeB", FORBIDDEN),
        ],
    );
}

#[test]
#[should_panic(expected = "duplicate OpenAPI operation ID: probeA")]
fn duplicate_openapi_id_fails() {
    check_catalog(
        &operations(&[
            ("/a", "probeA", Some("probe.a")),
            ("/b", "probeA", Some("probe.b")),
        ]),
        &[],
    );
}

#[test]
#[should_panic(expected = "duplicate domain operation name: probe.a")]
fn duplicate_domain_name_fails() {
    check_catalog(
        &two_operations(),
        &[
            entry("probe.a", "probeA", FORBIDDEN),
            entry("probe.a", "probeB", FORBIDDEN),
        ],
    );
}

#[test]
#[should_panic(expected = "surviving inferred handler ID: b_endpoint")]
fn surviving_inferred_handler_id_fails() {
    check_catalog(
        &operations(&[
            ("/a", "probeA", Some("probe.a")),
            ("/b", "b_endpoint", None),
        ]),
        &[
            entry("probe.a", "probeA", FORBIDDEN),
            entry("probe.b", "probeB", FORBIDDEN),
        ],
    );
}

#[test]
#[should_panic(expected = "probe.b is not bridged as probeB")]
fn missing_declared_operation_fails() {
    check_catalog(
        &operations(&[("/a", "probeA", Some("probe.a"))]),
        &[
            entry("probe.a", "probeA", FORBIDDEN),
            entry("probe.b", "probeB", FORBIDDEN),
        ],
    );
}

#[test]
fn shared_code_metadata_must_match() {
    for (field, changed) in [
        (
            "status",
            Mapping {
                status: 404,
                ..FORBIDDEN
            },
        ),
        (
            "kind",
            Mapping {
                kind: "refused",
                ..FORBIDDEN
            },
        ),
        (
            "message",
            Mapping {
                message: "Different.",
                ..FORBIDDEN
            },
        ),
        (
            "rule",
            Mapping {
                rule: Some("probe.rule"),
                ..FORBIDDEN
            },
        ),
        (
            "prerequisite",
            Mapping {
                prerequisite: Some("probe.prerequisite"),
                ..FORBIDDEN
            },
        ),
    ] {
        let result = std::panic::catch_unwind(|| {
            check_catalog(
                &two_operations(),
                &[
                    entry("probe.a", "probeA", FORBIDDEN),
                    entry("probe.b", "probeB", changed),
                ],
            )
        });
        let message = result
            .expect_err(field)
            .downcast::<String>()
            .map(|m| *m)
            .unwrap_or_default();
        assert_eq!(
            message, "public code probe.forbidden differs between probe.a and probe.b",
            "{field}"
        );
    }
}

#[derive(Clone, Copy)]
enum Never {}

fn never(n: Never) -> Mapping {
    match n {}
}

const PAGE: Mapping = Mapping {
    status: 200,
    kind: "success",
    code: None,
    message: "Page read",
    rule: None,
    prerequisite: None,
};

static READ: Operation<Never> = Operation {
    name: "probe.read",
    public_id: "probeRead",
    handler: "read_endpoint",
    method: Method::GET,
    success: PAGE,
    success_schema: "ProbePage",
    rejections: &[],
    rejection: never,
    recovery: None,
};

static WRITE: Operation<Never> = Operation {
    name: "probe.write",
    public_id: "probeWrite",
    handler: "write_endpoint",
    method: Method::POST,
    success: PAGE,
    success_schema: "ProbePage",
    rejections: &[],
    rejection: never,
    recovery: Some(Recovery {
        inspect: false,
        read: None,
        replay: false,
        new_submission: "probe",
    }),
};

static LINKED: Operation<Never> = Operation {
    name: "probe.linked",
    public_id: "probeLinked",
    handler: "linked_endpoint",
    method: Method::POST,
    success: PAGE,
    success_schema: "ProbePage",
    rejections: &[],
    rejection: never,
    recovery: Some(Recovery {
        inspect: false,
        read: Some(CurrentStateRead {
            operation: "probeRead",
            path_inputs: &[("project_id", "project_id"), ("team_id", "team")],
        }),
        replay: false,
        new_submission: "probe",
    }),
};

static TWICE: Operation<Never> = Operation {
    name: "probe.twice",
    public_id: "probeTwice",
    handler: "twice_endpoint",
    method: Method::POST,
    success: PAGE,
    success_schema: "ProbePage",
    rejections: &[],
    rejection: never,
    recovery: Some(Recovery {
        inspect: false,
        read: Some(CurrentStateRead {
            operation: "probeRead",
            path_inputs: &[("project_id", "project_id"), ("project_id", "user_id")],
        }),
        replay: false,
        new_submission: "probe",
    }),
};

/// Reports whether the boundary established request context for this request.
async fn seen(request: Request) -> Response {
    let id = request
        .extensions()
        .get::<RequestId>()
        .map_or("none".to_owned(), |id| id.as_str().to_owned());
    ([("x-seen", id.clone())], Json(json!({ "id": id }))).into_response()
}

async fn send(app: &Router, method: Method, path: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let request = Request::builder()
        .method(method)
        .uri(path)
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes().to_vec();
    (parts.status, parts.headers, bytes)
}

fn request_id(headers: &HeaderMap) -> &str {
    headers["x-seen"].to_str().unwrap()
}

fn is_request_id(value: &str) -> bool {
    value.len() == 36
        && value.starts_with("req_")
        && value[4..]
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

#[tokio::test]
async fn get_operation_boundary_serves_head_as_get() {
    let app = boundary(
        Router::new().route("/r", axum::routing::get(seen).post(seen)),
        &READ,
        |_| None,
    );
    let (status, get_headers, get_body) = send(&app, Method::GET, "/r").await;
    assert_eq!(status, StatusCode::OK);
    assert!(is_request_id(request_id(&get_headers)));
    assert!(!get_body.is_empty());
    let (status, head_headers, head_body) = send(&app, Method::HEAD, "/r").await;
    assert_eq!(status, StatusCode::OK);
    assert!(is_request_id(request_id(&head_headers)));
    assert!(head_body.is_empty(), "HEAD carries no body");
    for name in [header::CONTENT_TYPE, header::CONTENT_LENGTH] {
        assert_eq!(get_headers.get(&name), head_headers.get(&name), "{name}");
    }
    let (_, headers, _) = send(&app, Method::POST, "/r").await;
    assert_eq!(request_id(&headers), "none", "other methods stay outside");
}

#[tokio::test]
async fn post_operation_boundary_leaves_get_and_head_outside() {
    let app = boundary(
        Router::new().route("/w", axum::routing::get(seen).post(seen)),
        &WRITE,
        |_| None,
    );
    let (_, headers, _) = send(&app, Method::POST, "/w").await;
    assert!(is_request_id(request_id(&headers)));
    for method in [Method::GET, Method::HEAD] {
        let (_, headers, _) = send(&app, method.clone(), "/w").await;
        assert_eq!(request_id(&headers), "none", "{method}");
    }
}

fn bridged(op: &Operation<Never>, method: &str) -> Value {
    let mut api = document(
        json!({"/p": {method: {"operationId": op.handler, "responses": {}}}}),
        json!({}),
    );
    let page = RefOr::T(Schema::Object(utoipa::openapi::schema::Object::new()));
    bridge(&mut api, op, vec![("ProbePage".into(), page)]);
    let doc = serde_json::to_value(&api).unwrap();
    doc["paths"]["/p"][method].clone()
}

#[test]
fn reads_declare_no_recovery() {
    let operation = bridged(&READ, "get");
    assert_eq!(operation["operationId"], "probeRead");
    assert_eq!(
        operation["x-iris"],
        json!({"operation": "probe.read", "schema_version": 1, "prerequisites": {}})
    );
    // No CSRF refusal for a safe method, so no 403 at all here.
    let statuses = operation["responses"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(statuses, ["200", "400", "401", "500", "503"]);
}

#[test]
fn writes_keep_their_recovery() {
    let operation = bridged(&WRITE, "post");
    assert_eq!(
        operation["x-iris"]["recovery"],
        json!({"inspect": false, "read": false, "replay": false, "new_submission": "probe"})
    );
    assert!(operation["responses"]["403"].is_object(), "CSRF refusal");
}

#[test]
fn writes_render_their_current_state_read() {
    let operation = bridged(&LINKED, "post");
    assert_eq!(
        operation["x-iris"]["recovery"],
        json!({
            "inspect": false,
            "read": {
                "operation_id": "probeRead",
                "path_inputs": {
                    "project_id": {"request_body_field": "project_id"},
                    "team_id": {"request_body_field": "team"},
                },
            },
            "replay": false,
            "new_submission": "probe",
        })
    );
}

#[test]
#[should_panic(expected = "probe.twice binds path parameter project_id twice")]
fn a_path_parameter_bound_twice_fails() {
    bridged(&TWICE, "post");
}

fn id() -> Value {
    json!({"type": "string", "maxLength": 19, "pattern": "^[1-9][0-9]*$"})
}

/// A write that declares `probeRead` as its current-state read, and that read.
/// Each case below changes one part.
fn linked() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {"title": "probe", "version": "1"},
        "paths": {
            "/write": {"post": {
                "operationId": "probeWrite",
                "requestBody": {"content": {"application/json": {
                    "schema": {"$ref": "#/components/schemas/ProbeRequest"},
                }}},
                "responses": {},
                "x-iris": {"operation": "probe.write", "recovery": {
                    "inspect": false,
                    "read": {
                        "operation_id": "probeRead",
                        "path_inputs": {"project_id": {"request_body_field": "project_id"}},
                    },
                    "replay": false,
                    "new_submission": "probe",
                }},
            }},
            "/projects/{project_id}/items": {"get": {
                "operationId": "probeRead",
                "parameters": [
                    {"name": "project_id", "in": "path", "required": true, "schema": id()},
                    {"name": "limit", "in": "query", "required": false, "schema": {"type": "string"}},
                ],
                "responses": {},
                "x-iris": {"operation": "probe.read"},
            }},
        },
        "components": {"schemas": {"ProbeRequest": {
            "type": "object",
            "required": ["project_id", "user_id"],
            "properties": {"project_id": id(), "user_id": id()},
        }}},
    })
}

fn source(doc: &mut Value) -> &mut Value {
    &mut doc["paths"]["/write"]["post"]
}

fn declared(doc: &mut Value) -> &mut Value {
    &mut source(doc)["x-iris"]["recovery"]["read"]
}

fn target(doc: &mut Value) -> &mut Value {
    &mut doc["paths"]["/projects/{project_id}/items"]["get"]
}

fn check_linked(doc: Value) {
    check_catalog(&serde_json::from_value(doc).unwrap(), &[]);
}

#[test]
fn consistent_current_state_read_passes() {
    check_linked(linked());
}

#[test]
fn inline_request_body_schema_passes() {
    let mut doc = linked();
    let schema = doc["components"]["schemas"]["ProbeRequest"].clone();
    source(&mut doc)["requestBody"]["content"]["application/json"]["schema"] = schema;
    check_linked(doc);
}

#[test]
fn undeclared_read_is_not_linked() {
    let mut doc = linked();
    *declared(&mut doc) = json!(false);
    *target(&mut doc) = json!({"operationId": "probeRead", "responses": {}});
    check_linked(doc);
}

#[test]
#[should_panic(expected = "probe.write declares an unsupported recovery read")]
fn a_read_declared_as_true_fails() {
    let mut doc = linked();
    *declared(&mut doc) = json!(true);
    check_linked(doc);
}

#[test]
#[should_panic(expected = "probe.write declares an unsupported recovery read")]
fn an_extra_descriptor_field_fails() {
    let mut doc = linked();
    declared(&mut doc)["cursor"] = json!("first");
    check_linked(doc);
}

#[test]
#[should_panic(expected = "probe.write declares an unsupported recovery read")]
fn an_unsupported_binding_shape_fails() {
    let mut doc = linked();
    declared(&mut doc)["path_inputs"]["project_id"]["constant"] = json!("41");
    check_linked(doc);
}

#[test]
#[should_panic(expected = "probe.write recovery read probeMissing is not in the document")]
fn an_unresolved_read_fails() {
    let mut doc = linked();
    declared(&mut doc)["operation_id"] = json!("probeMissing");
    check_linked(doc);
}

#[test]
#[should_panic(expected = "probe.write recovery read probeRead is not an Iris operation")]
fn a_read_outside_the_catalog_fails() {
    let mut doc = linked();
    target(&mut doc).as_object_mut().unwrap().remove("x-iris");
    check_linked(doc);
}

#[test]
#[should_panic(expected = "probe.write recovery read probeRead is not a GET operation")]
fn a_post_target_fails() {
    let mut doc = linked();
    let read = target(&mut doc).clone();
    doc["paths"]["/projects/{project_id}/items"] = json!({"post": read});
    check_linked(doc);
}

#[test]
#[should_panic(expected = "probe.write recovery read probeRead declares recovery")]
fn a_target_with_recovery_fails() {
    let mut doc = linked();
    target(&mut doc)["x-iris"]["recovery"] =
        json!({"inspect": false, "read": false, "replay": false, "new_submission": "probe"});
    check_linked(doc);
}

#[test]
#[should_panic(
    expected = "probe.write recovery read leaves probeRead's path parameter project_id unbound"
)]
fn a_missing_binding_fails() {
    let mut doc = linked();
    declared(&mut doc)["path_inputs"] = json!({});
    check_linked(doc);
}

#[test]
#[should_panic(
    expected = "probe.write recovery read leaves probeRead's required query parameter limit unbound"
)]
fn an_unbound_required_query_parameter_fails() {
    let mut doc = linked();
    target(&mut doc)["parameters"][1]["required"] = json!(true);
    check_linked(doc);
}

/// A path binding supplies only the path parameter of that name.
#[test]
fn a_required_parameter_sharing_a_bound_name_fails() {
    for location in ["query", "header", "cookie"] {
        let mut doc = linked();
        target(&mut doc)["parameters"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name": "project_id", "in": location, "required": true, "schema": id()}));
        let message = std::panic::catch_unwind(|| check_linked(doc))
            .expect_err(location)
            .downcast::<String>()
            .map(|m| *m)
            .unwrap_or_default();
        assert_eq!(
            message,
            format!(
                "probe.write recovery read leaves probeRead's required {location} parameter project_id unbound"
            ),
            "{location}"
        );
    }
}

#[test]
#[should_panic(
    expected = "probe.write recovery read binds limit, which is not a path parameter of probeRead"
)]
fn an_extra_binding_fails() {
    let mut doc = linked();
    declared(&mut doc)["path_inputs"]["limit"] = json!({"request_body_field": "user_id"});
    check_linked(doc);
}

#[test]
#[should_panic(expected = "probe.write recovery read needs a JSON object request body")]
fn a_missing_request_body_fails() {
    let mut doc = linked();
    source(&mut doc)
        .as_object_mut()
        .unwrap()
        .remove("requestBody");
    check_linked(doc);
}

#[test]
#[should_panic(expected = "probe.write recovery read needs a JSON object request body")]
fn a_non_local_request_body_reference_fails() {
    let mut doc = linked();
    source(&mut doc)["requestBody"]["content"]["application/json"]["schema"] =
        json!({"$ref": "other.json#/components/schemas/ProbeRequest"});
    check_linked(doc);
}

#[test]
#[should_panic(
    expected = "probe.write recovery read binds project_id from team_id, which the request body does not declare"
)]
fn an_unknown_body_field_fails() {
    let mut doc = linked();
    declared(&mut doc)["path_inputs"]["project_id"]["request_body_field"] = json!("team_id");
    check_linked(doc);
}

#[test]
#[should_panic(
    expected = "probe.write recovery read binds project_id from project_id, which the request body does not require"
)]
fn an_optional_body_field_fails() {
    let mut doc = linked();
    doc["components"]["schemas"]["ProbeRequest"]["required"] = json!(["user_id"]);
    check_linked(doc);
}

#[test]
#[should_panic(
    expected = "probe.write recovery read binds project_id from project_id, whose schema differs"
)]
fn a_differing_field_schema_fails() {
    let mut doc = linked();
    doc["components"]["schemas"]["ProbeRequest"]["properties"]["project_id"]["maxLength"] =
        json!(20);
    check_linked(doc);
}
