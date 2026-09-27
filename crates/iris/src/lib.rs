//! Provisional, unpublished Iris framework code. Presence here is not a
//! stability promise, and the crate name is not settled.
//!
//! Extracted from the S17 reference application once `memberships.change_role`
//! and `memberships.remove_member` both needed it: envelope rendering, the
//! per-operation response bridge, the shared refusal/failure profile, the
//! request-ID boundary and assembly checks. Applications keep transactions,
//! cleanup classification, authorization, SQL, domain types and sessions.
//!
//! `Operation` declares a response contract for the bridge; it does not wrap
//! or replace route registration, which stays an explicit `routes!` call.
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

pub const VERSION: u32 = 1;

/// Inserted by [`boundary`] for its operation's handler.
#[derive(Clone)]
pub struct RequestId(String);

impl RequestId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, strum::VariantArray)]
pub enum Shared {
    Invalid,
    Unauthenticated,
    Csrf,
    Internal,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mapping {
    pub status: u16,
    pub kind: &'static str,
    pub code: Option<&'static str>,
    pub message: &'static str,
    /// Compared across operations; never exported.
    pub rule: Option<&'static str>,
    pub prerequisite: Option<&'static str>,
}

pub fn shared(s: Shared) -> Mapping {
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

pub struct Recovery {
    pub inspect: bool,
    /// Exported as `false` when absent.
    pub read: Option<CurrentStateRead>,
    pub replay: bool,
    pub new_submission: &'static str,
}

/// A read of present state that an operation declares for recovery (S15).
/// After an unresolved attempt, a caller may send it under their current
/// authorization, as a fresh first page with no cursor; the binding supplies
/// no query parameters. What it returns establishes no outcome for that
/// attempt, no causality and no concurrency fence, and it authorizes no new
/// submission.
pub struct CurrentStateRead {
    /// The read's public operation ID.
    pub operation: &'static str,
    /// Each of the read's path parameters, with the request-body field that
    /// supplies it.
    pub path_inputs: &'static [(&'static str, &'static str)],
}

/// One operation's public contract. Rendering, export and assembly checks all
/// read these fields, so they cannot drift apart.
pub struct Operation<R: 'static> {
    pub name: &'static str,
    pub public_id: &'static str,
    /// The identity utoipa infers from the handler, replaced by the bridge.
    pub handler: &'static str,
    pub method: Method,
    pub success: Mapping,
    pub success_schema: &'static str,
    /// Every rejection variant, so the mapping below is exhaustive.
    pub rejections: &'static [R],
    pub rejection: fn(R) -> Mapping,
    /// `None` for reads, which declare no recovery capabilities: repeating a
    /// read is a new observation, not a replay.
    pub recovery: Option<Recovery>,
}

impl<R: Copy> Operation<R> {
    /// Success, every rejection, then the shared profile for this method.
    pub fn mappings(&self) -> Vec<Mapping> {
        let csrf = !csrf_exempt(&self.method);
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

    pub fn render(&self, mapping: &Mapping, id: &RequestId, data: Option<Value>) -> Response {
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

/// Safe methods skip CSRF. A session layer and the operation profile must
/// share this rule, so both call it.
pub fn csrf_exempt(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// A success projection's named schema followed by its dependencies.
pub fn success_schemas<S: utoipa::ToSchema>() -> Vec<(String, RefOr<Schema>)> {
    let mut schemas = vec![(S::name().into_owned(), S::schema())];
    S::schemas(&mut schemas);
    schemas
}

/// What assembly checks across operations.
pub struct CatalogEntry {
    pub name: &'static str,
    pub public_id: &'static str,
    pub handler: &'static str,
    pub mappings: Vec<Mapping>,
}

/// One bridged operation, collected alone.
pub struct Collected<S> {
    pub router: Router<S>,
    pub api: OpenApi,
    pub entry: CatalogEntry,
}

pub fn collect<S: Clone + Send + Sync + 'static, R: Copy>(
    op: &'static Operation<R>,
    mut router: OpenApiRouter<S>,
    success: Vec<(String, RefOr<Schema>)>,
) -> Collected<S> {
    bridge(router.get_openapi_mut(), op, success);
    let (router, api) = router.split_for_parts();
    Collected {
        router,
        api,
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
    let mut iris = serde_json::Map::new();
    iris.insert("operation".into(), json!(op.name));
    iris.insert("schema_version".into(), json!(VERSION));
    if let Some(recovery) = &op.recovery {
        let read = recovery.read.as_ref().map_or(json!(false), |read| {
            let mut inputs = serde_json::Map::new();
            for (param, field) in read.path_inputs {
                assert!(
                    inputs
                        .insert(param.to_string(), json!({"request_body_field": field}))
                        .is_none(),
                    "{} binds path parameter {param} twice",
                    op.name
                );
            }
            json!({"operation_id": read.operation, "path_inputs": inputs})
        });
        iris.insert("recovery".into(), json!({"inspect":recovery.inspect,"read":read,"replay":recovery.replay,"new_submission":recovery.new_submission}));
    }
    iris.insert(
        "prerequisites".into(),
        json!(
            mappings
                .iter()
                .filter_map(|m| m.code.zip(m.prerequisite))
                .collect::<BTreeMap<_, _>>()
        ),
    );
    operation.extensions = Some(serde_json::from_value(json!({ "x-iris": iris })).unwrap());
    assert_eq!(
        success.first().map(|(name, _)| name.as_str()),
        Some(op.success_schema),
        "{} success projection does not match its declared schema",
        op.name
    );
    // Security schemes belong to the application's assembled document.
    api.components
        .get_or_insert_with(Default::default)
        .schemas
        .extend(success);
}

/// Apply only to one collected operation's router. The method check keeps
/// method-not-allowed outside this operation's contract. Axum serves HEAD
/// through a GET route, so a GET operation's boundary also covers HEAD: the
/// same status and headers, with the body stripped by axum. `classify` maps the
/// application's own refusal markers on inner responses to the shared profile.
pub fn boundary<S: Clone + Send + Sync + 'static, R: Copy + Send + Sync>(
    router: Router<S>,
    op: &'static Operation<R>,
    classify: fn(&Response) -> Option<Shared>,
) -> Router<S> {
    router.route_layer(middleware::from_fn(
        move |mut request: Request, next: Next| async move {
            let head_as_get = op.method == Method::GET && request.method() == Method::HEAD;
            if request.method() != op.method && !head_as_get {
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
            if let Some(mapping) = classify(&response) {
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
pub fn merge_checked(into: &mut OpenApi, from: OpenApi) {
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

/// Checks the assembled document against every declared operation, and each
/// declared current-state read against its target.
pub fn check_catalog(api: &OpenApi, entries: &[CatalogEntry]) {
    let doc = serde_json::to_value(api).unwrap();
    let mut operations = HashMap::new();
    for item in doc["paths"]
        .as_object()
        .into_iter()
        .flat_map(|p| p.values())
    {
        for (method, operation) in item.as_object().into_iter().flatten() {
            if let Some(id) = operation["operationId"].as_str() {
                assert!(
                    operations
                        .insert(id, (method.as_str(), operation))
                        .is_none(),
                    "duplicate OpenAPI operation ID: {id}"
                );
            }
        }
    }
    for (_, operation) in operations.values() {
        if operation["x-iris"]["recovery"].is_object() {
            check_current_state_read(&doc, &operations, operation);
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
            !operations.contains_key(entry.handler),
            "surviving inferred handler ID: {}",
            entry.handler
        );
        assert_eq!(
            operations
                .get(entry.public_id)
                .map(|(_, operation)| operation["x-iris"]["operation"].as_str()),
            Some(Some(entry.name)),
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

/// The target must be an Iris GET without recovery that requires no parameter
/// outside its path. The bindings must cover exactly its path parameters, each
/// from a required request-body field with an identical schema. Equality is
/// deliberately conservative: it also rejects some compatible schemas.
/// Structure cannot show that a binding names the right field; an independent
/// test must.
fn check_current_state_read(
    doc: &Value,
    operations: &HashMap<&str, (&str, &Value)>,
    source: &Value,
) {
    let name = source["x-iris"]["operation"].as_str().unwrap_or_default();
    let read = &source["x-iris"]["recovery"]["read"];
    if *read == json!(false) {
        return;
    }
    let Some((id, bindings)) = descriptor(read) else {
        panic!("{name} declares an unsupported recovery read");
    };
    let Some((method, target)) = operations.get(id) else {
        panic!("{name} recovery read {id} is not in the document");
    };
    assert!(
        target["x-iris"].is_object(),
        "{name} recovery read {id} is not an Iris operation"
    );
    assert!(
        *method == "get",
        "{name} recovery read {id} is not a GET operation"
    );
    assert!(
        target["x-iris"].get("recovery").is_none(),
        "{name} recovery read {id} declares recovery"
    );
    let parameters = target["parameters"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let path = |param: &str| {
        parameters
            .iter()
            .find(|p| p["in"] == "path" && p["name"] == param)
    };
    // Bindings supply path parameters only, so a required parameter elsewhere
    // is never bound, whatever its name.
    for parameter in parameters {
        let param = parameter["name"].as_str().unwrap_or_default();
        if parameter["in"] == "path" {
            assert!(
                bindings.iter().any(|(bound, _)| *bound == param),
                "{name} recovery read leaves {id}'s path parameter {param} unbound"
            );
        } else {
            assert!(
                parameter["required"] != true,
                "{name} recovery read leaves {id}'s required {} parameter {param} unbound",
                parameter["in"].as_str().unwrap_or_default()
            );
        }
    }
    for (param, _) in &bindings {
        assert!(
            path(param).is_some(),
            "{name} recovery read binds {param}, which is not a path parameter of {id}"
        );
    }
    let Some(body) = request_schema(doc, source) else {
        panic!("{name} recovery read needs a JSON object request body");
    };
    for (param, field) in &bindings {
        let declared = &body["properties"][field];
        assert!(
            !declared.is_null(),
            "{name} recovery read binds {param} from {field}, which the request body does not declare"
        );
        assert!(
            body["required"]
                .as_array()
                .is_some_and(|required| required.iter().any(|r| r.as_str() == Some(field))),
            "{name} recovery read binds {param} from {field}, which the request body does not require"
        );
        assert!(
            *declared == path(param).unwrap()["schema"],
            "{name} recovery read binds {param} from {field}, whose schema differs"
        );
    }
}

/// `{operation_id, path_inputs: {<parameter>: {request_body_field}}}`, and
/// nothing else.
fn descriptor(read: &Value) -> Option<(&str, Vec<(&str, &str)>)> {
    let read = read.as_object()?;
    if read.len() != 2 {
        return None;
    }
    let id = read.get("operation_id")?.as_str()?;
    let bindings = read
        .get("path_inputs")?
        .as_object()?
        .iter()
        .map(|(param, input)| {
            let input = input.as_object().filter(|input| input.len() == 1)?;
            Some((param.as_str(), input.get("request_body_field")?.as_str()?))
        })
        .collect::<Option<_>>()?;
    Some((id, bindings))
}

/// The JSON request-body schema: inline, or one local component reference.
fn request_schema<'a>(doc: &'a Value, source: &'a Value) -> Option<&'a Value> {
    let schema = &source["requestBody"]["content"]["application/json"]["schema"];
    let schema = match schema.get("$ref") {
        Some(reference) => doc["components"]["schemas"]
            .get(reference.as_str()?.strip_prefix("#/components/schemas/")?)?,
        None => schema,
    };
    (schema["type"] == "object").then_some(schema)
}

#[cfg(test)]
mod tests;
