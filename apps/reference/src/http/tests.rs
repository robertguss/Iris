//! Assembly checks on synthetic documents; source-copy omission probes are separate.
use super::*;

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
