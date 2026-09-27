use iris_reference::app::openapi;
use serde_json::Value;

#[test]
fn exported_document_is_current() {
    let snapshot = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("openapi.json");
    let current = openapi().to_pretty_json().unwrap() + "\n";
    assert!(
        std::fs::read_to_string(&snapshot).unwrap() == current,
        "reference contract drift: {}; run cargo run -p iris-reference --bin export-openapi -- apps/reference/openapi.json",
        snapshot.display()
    );
}

/// Written by hand, not derived from the document under test.
#[test]
fn independent_inventory() {
    let doc = serde_json::to_value(openapi()).unwrap();
    let mut actual = vec![];
    for (path, item) in doc["paths"].as_object().unwrap() {
        for (method, operation) in item.as_object().unwrap() {
            actual.push((
                method.as_str(),
                path.as_str(),
                operation["operationId"].as_str().unwrap(),
            ));
        }
    }
    actual.sort();
    let mut expected = vec![
        ("get", "/api/auth/session", "session_info"),
        ("post", "/api/auth/login", "login"),
        ("get", "/api/auth/callback", "callback"),
        ("post", "/api/auth/logout", "logout"),
        ("post", "/api/memberships/role", "changeMemberRole"),
        ("post", "/api/memberships/remove", "removeMember"),
    ];
    expected.sort();
    assert_eq!(actual, expected);
    let collected = iris_reference::http::memberships::collect();
    assert_eq!(collected.len(), 2);
    for operation in collected {
        let operation: Value = serde_json::to_value(operation.api).unwrap();
        for (path, item) in operation["paths"].as_object().unwrap() {
            assert_eq!(
                &doc["paths"][path], item,
                "merging must not alter the collected operation"
            );
        }
    }
}
