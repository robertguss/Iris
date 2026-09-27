//! Envelope rendering, per-operation bridging, the request-ID boundary and
//! assembly checks, generalized from S16 for more than one operation.
pub mod memberships;

use crate::{
    app::AppState,
    identity::{self, Auth, ErrorCode},
};
use axum::{
    Json, Router,
    extract::Request,
    http::{Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use strum::VariantArray;
use utoipa::openapi::{OpenApi, RefOr, schema::Schema};
use utoipa_axum::router::OpenApiRouter;

pub(crate) const VERSION: u32 = 1;

#[derive(Clone)]
pub(crate) struct RequestId(pub(crate) String);

#[derive(Clone, Copy, strum::VariantArray)]
pub(crate) enum Shared {
    Invalid,
    Unauthenticated,
    Csrf,
    Internal,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Mapping {
    pub(crate) status: u16,
    pub(crate) kind: &'static str,
    pub(crate) code: Option<&'static str>,
    pub(crate) message: &'static str,
    /// Compared across operations; never exported.
    pub(crate) rule: Option<&'static str>,
    pub(crate) prerequisite: Option<&'static str>,
}

pub(crate) fn shared(s: Shared) -> Mapping {
    let (status, kind, code, message) = match s {
        Shared::Invalid => (
            400,
            "refused",
            "http.invalid_request",
            "Provide valid request fields.",
        ),
        Shared::Unauthenticated => (
            401,
            "refused",
            "http.unauthenticated",
            "Sign in to continue.",
        ),
        Shared::Csrf => (
            403,
            "refused",
            "http.csrf_refused",
            "Refresh the session and provide its CSRF token.",
        ),
        Shared::Internal => (
            500,
            "failure",
            "iris.internal",
            "A normal response could not be produced.",
        ),
        Shared::Unavailable => (
            503,
            "failure",
            "iris.unavailable",
            "The service is unavailable.",
        ),
    };
    Mapping {
        status,
        kind,
        code: Some(code),
        message,
        rule: None,
        prerequisite: None,
    }
}

pub(crate) struct Recovery {
    pub(crate) inspect: bool,
    pub(crate) read: bool,
    pub(crate) replay: bool,
    pub(crate) new_submission: &'static str,
}

/// One operation's public contract. Rendering, export and assembly checks all
/// read these fields, so they cannot drift apart.
pub(crate) struct Operation<R: 'static> {
    pub(crate) name: &'static str,
    pub(crate) public_id: &'static str,
    /// The identity utoipa infers from the handler, replaced by the bridge.
    pub(crate) handler: &'static str,
    pub(crate) method: Method,
    pub(crate) success: Mapping,
    pub(crate) success_schema: &'static str,
    /// Every rejection variant, so the mapping below is exhaustive.
    pub(crate) rejections: &'static [R],
    pub(crate) rejection: fn(R) -> Mapping,
    pub(crate) recovery: Recovery,
}

impl<R: Copy> Operation<R> {
    /// Success, every rejection, then the shared profile for this method.
    pub(crate) fn mappings(&self) -> Vec<Mapping> {
        let csrf = !identity::csrf_exempt(&self.method);
        std::iter::once(self.success)
            .chain(self.rejections.iter().copied().map(self.rejection))
            .chain(
                Shared::VARIANTS
                    .iter()
                    .copied()
                    .filter(|s| csrf || !matches!(s, Shared::Csrf))
                    .map(shared),
            )
            .collect()
    }

    pub(crate) fn render(
        &self,
        mapping: &Mapping,
        id: &RequestId,
        data: Option<Value>,
    ) -> Response {
        let mut body = json!({"schema_version": VERSION, "operation": self.name, "request_id": id.0, "kind": mapping.kind});
        if let Some(code) = mapping.code {
            body["code"] = json!(code);
            body["message"] = json!(mapping.message);
        } else {
            body["data"] = data.expect("success projection");
        }
        (StatusCode::from_u16(mapping.status).unwrap(), Json(body)).into_response()
    }

    fn schema(&self, mapping: &Mapping) -> Value {
        let mut schema = json!({"type":"object", "required":["schema_version","operation","request_id","kind"], "properties": {
            "schema_version":{"type":"integer","enum":[VERSION]},
            "operation":{"type":"string","enum":[self.name]},
            "request_id":{"type":"string","pattern":"^req_[0-9a-f]{32}$"},
            "kind":{"type":"string","enum":[mapping.kind]}
        }});
        let required = schema["required"].as_array_mut().unwrap();
        if let Some(code) = mapping.code {
            required.extend([json!("code"), json!("message")]);
            schema["properties"]["code"] = json!({"type":"string","enum":[code]});
            schema["properties"]["message"] = json!({"type":"string"});
        } else {
            required.push(json!("data"));
            schema["properties"]["data"] =
                json!({"$ref": format!("#/components/schemas/{}", self.success_schema)});
        }
        schema
    }
}

/// Positive canonical decimal IDs only; the caller maps failure to its refusal.
pub(crate) fn parse_id(value: &str) -> Result<i64, ()> {
    let parsed = value
        .parse::<i64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == value);
    parsed.ok_or(())
}

/// A success projection's named schema followed by its dependencies.
pub(crate) fn success_schemas<S: utoipa::ToSchema>() -> Vec<(String, RefOr<Schema>)> {
    let mut schemas = vec![(S::name().into_owned(), S::schema())];
    S::schemas(&mut schemas);
    schemas
}

/// Applies the session stack, then this operation's boundary, to its router.
pub(crate) type Mount = fn(Router<AppState>, Auth) -> Router<AppState>;

/// What assembly checks across operations.
pub(crate) struct CatalogEntry {
    pub(crate) name: &'static str,
    pub(crate) public_id: &'static str,
    pub(crate) handler: &'static str,
    pub(crate) mappings: Vec<Mapping>,
}

/// One bridged operation, collected alone.
pub struct Collected {
    pub(crate) router: Router<AppState>,
    pub api: OpenApi,
    pub(crate) mount: Mount,
    pub(crate) entry: CatalogEntry,
}

pub(crate) fn collect<R: Copy>(
    op: &'static Operation<R>,
    mut router: OpenApiRouter<AppState>,
    success: Vec<(String, RefOr<Schema>)>,
    mount: Mount,
) -> Collected {
    bridge(router.get_openapi_mut(), op, success);
    let (router, api) = router.split_for_parts();
    Collected {
        router,
        api,
        mount,
        entry: CatalogEntry {
            name: op.name,
            public_id: op.public_id,
            handler: op.handler,
            mappings: op.mappings(),
        },
    }
}

fn bridge<R: Copy>(api: &mut OpenApi, op: &Operation<R>, success: Vec<(String, RefOr<Schema>)>) {
    assert_eq!(
        api.paths.paths.len(),
        1,
        "{} requires exactly one collected path",
        op.name
    );
    let item = api.paths.paths.values_mut().next().unwrap();
    let value = serde_json::to_value(&*item).unwrap();
    assert_eq!(
        value
            .as_object()
            .unwrap()
            .keys()
            .filter(|k| [
                "get", "post", "put", "patch", "delete", "head", "options", "trace"
            ]
            .contains(&k.as_str()))
            .count(),
        1,
        "{} requires exactly one operation",
        op.name
    );
    let operation = match op.method {
        Method::GET => item.get.as_mut(),
        Method::POST => item.post.as_mut(),
        _ => None,
    }
    .unwrap_or_else(|| panic!("{} requires a collected {}", op.name, op.method));
    assert_eq!(
        operation.operation_id.as_deref(),
        Some(op.handler),
        "{} unexpected collected handler identity",
        op.name
    );
    operation.operation_id = Some(op.public_id.into());
    let mut groups = BTreeMap::<String, Vec<Value>>::new();
    let mut codes = HashSet::new();
    let mappings = op.mappings();
    for mapping in &mappings {
        if let Some(code) = mapping.code {
            assert!(codes.insert(code), "duplicate public code");
        }
        groups
            .entry(mapping.status.to_string())
            .or_default()
            .push(op.schema(mapping));
    }
    operation.responses = serde_json::from_value(json!(groups.into_iter().map(|(status, branches)| (status, json!({"description":"Iris response", "content":{"application/json":{"schema":{"oneOf":branches}}}}))).collect::<BTreeMap<_,_>>())).unwrap();
    let recovery = &op.recovery;
    operation.extensions = Some(serde_json::from_value(json!({"x-iris": {
        "operation":op.name, "schema_version":VERSION,
        "recovery":{"inspect":recovery.inspect,"read":recovery.read,"replay":recovery.replay,"new_submission":recovery.new_submission},
        "prerequisites":mappings.iter().filter_map(|m| m.code.zip(m.prerequisite)).collect::<BTreeMap<_,_>>()
    }})).unwrap());
    assert_eq!(
        success.first().map(|(name, _)| name.as_str()),
        Some(op.success_schema),
        "{} success projection does not match its declared schema",
        op.name
    );
    let components = api.components.get_or_insert_with(Default::default);
    components.add_security_scheme("BrowserSession", identity::security_scheme());
    components.schemas.extend(success);
}

/// Apply only to one collected operation's router. The method check keeps
/// method-not-allowed outside this operation's contract.
pub(crate) fn boundary<R: Copy + Send + Sync>(
    router: Router<AppState>,
    op: &'static Operation<R>,
) -> Router<AppState> {
    router.route_layer(middleware::from_fn(
        move |mut request: Request, next: Next| async move {
            if request.method() != op.method {
                return next.run(request).await;
            }
            let mut bytes = [0u8; 16];
            if getrandom::fill(&mut bytes).is_err() {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            let id = RequestId(format!(
                "req_{}",
                bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
            ));
            request.extensions_mut().insert(id.clone());
            let response = next.run(request).await;
            let mapping = match response.extensions().get::<ErrorCode>() {
                Some(ErrorCode::Csrf) => Some(Shared::Csrf),
                Some(ErrorCode::Unauthorized) => Some(Shared::Unauthenticated),
                Some(ErrorCode::Internal) => Some(Shared::Internal),
                _ => None,
            };
            if let Some(mapping) = mapping {
                let mut rendered = op.render(&shared(mapping), &id, None);
                // Preserve cookies/no-store, not the old body's length or MIME type.
                for (name, value) in response.headers() {
                    if name != header::CONTENT_TYPE && name != header::CONTENT_LENGTH {
                        rendered.headers_mut().append(name, value.clone());
                    }
                }
                rendered
            } else {
                response
            }
        },
    ))
}

/// Fails assembly when a same-named component or a path would be silently
/// replaced; utoipa's merge keeps the first definition.
pub(crate) fn merge_checked(into: &mut OpenApi, from: OpenApi) {
    for path in from.paths.paths.keys() {
        assert!(
            !into.paths.paths.contains_key(path),
            "path conflict: {path}"
        );
    }
    let existing = serde_json::to_value(&into.components).unwrap();
    let incoming = serde_json::to_value(&from.components).unwrap();
    if let (Some(existing), Some(incoming)) = (existing.as_object(), incoming.as_object()) {
        for (category, entries) in incoming {
            for (name, definition) in entries.as_object().into_iter().flatten() {
                if let Some(current) = existing.get(category).and_then(|c| c.get(name)) {
                    assert!(
                        current == definition,
                        "component conflict: {category}/{name}"
                    );
                }
            }
        }
    }
    into.merge(from);
}

/// Checks the assembled document against every declared operation.
pub(crate) fn check_catalog(api: &OpenApi, entries: &[CatalogEntry]) {
    let doc = serde_json::to_value(api).unwrap();
    let mut ids = HashMap::new();
    for item in doc["paths"]
        .as_object()
        .into_iter()
        .flat_map(|p| p.values())
    {
        for operation in item.as_object().into_iter().flat_map(|i| i.values()) {
            if let Some(id) = operation["operationId"].as_str() {
                assert!(
                    ids.insert(id, operation["x-iris"]["operation"].as_str())
                        .is_none(),
                    "duplicate OpenAPI operation ID: {id}"
                );
            }
        }
    }
    let mut names = HashSet::new();
    let mut codes = HashMap::<&str, (&Mapping, &str)>::new();
    for entry in entries {
        assert!(
            names.insert(entry.name),
            "duplicate domain operation name: {}",
            entry.name
        );
        assert!(
            !ids.contains_key(entry.handler),
            "surviving inferred handler ID: {}",
            entry.handler
        );
        assert_eq!(
            ids.get(entry.public_id),
            Some(&Some(entry.name)),
            "{} is not bridged as {}",
            entry.name,
            entry.public_id
        );
        for mapping in &entry.mappings {
            let Some(code) = mapping.code else { continue };
            match codes.get(code) {
                Some((first, owner)) => assert!(
                    *first == mapping,
                    "public code {code} differs between {owner} and {}",
                    entry.name
                ),
                None => {
                    codes.insert(code, (mapping, entry.name));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
