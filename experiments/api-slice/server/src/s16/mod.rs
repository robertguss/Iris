//! Isolated S16 experiment. Not registered in either existing demo router.
pub mod action;

use crate::{
    Actor, AppState, ErrorCode,
    auth::Auth,
    members::{ChangeRoleRequest, Role},
};
use action::{Acknowledged, ActionError, FailureKind, Rejection, StopReason};
use axum::{
    Extension, Json, Router,
    extract::{Request, State, rejection::JsonRejection},
    http::{Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use strum::VariantArray;

const OPERATION: &str = "memberships.change_role";
const PUBLIC_ID: &str = "changeMemberRole";
const VERSION: u32 = 1;

#[derive(Clone)]
struct RequestId(String);

#[derive(Clone, Copy, strum::VariantArray)]
enum Shared {
    Invalid,
    Unauthenticated,
    Csrf,
    Internal,
    Unavailable,
}

struct Mapping {
    status: u16,
    kind: &'static str,
    code: Option<&'static str>,
    message: &'static str,
}

fn rejection(r: Rejection) -> Mapping {
    let status = match r {
        Rejection::Forbidden => 403,
        Rejection::MemberNotFound => 404,
        Rejection::LastOwner => 409,
    };
    let descriptor = r.descriptor();
    Mapping {
        status,
        kind: "rejected",
        code: Some(descriptor.code),
        message: descriptor.summary,
    }
}

fn shared(s: Shared) -> Mapping {
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
    }
}

fn success() -> Mapping {
    Mapping {
        status: 200,
        kind: "success",
        code: None,
        message: "Commit acknowledged",
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
enum Completion {
    Acknowledged,
}
#[derive(serde::Serialize, utoipa::ToSchema)]
struct SuccessData {
    completion: Completion,
}
fn project(_: Acknowledged) -> SuccessData {
    SuccessData {
        completion: Completion::Acknowledged,
    }
}

impl Mapping {
    fn render(&self, id: &RequestId, data: Option<SuccessData>) -> Response {
        let mut body = json!({"schema_version": VERSION, "operation": OPERATION, "request_id": id.0, "kind": self.kind});
        if let Some(code) = self.code {
            body["code"] = json!(code);
            body["message"] = json!(self.message);
        } else {
            body["data"] = serde_json::to_value(data.expect("success projection")).unwrap();
        }
        (StatusCode::from_u16(self.status).unwrap(), Json(body)).into_response()
    }

    fn schema(&self) -> Value {
        let mut schema = json!({"type":"object", "required":["schema_version","operation","request_id","kind"], "properties": {
            "schema_version":{"type":"integer","enum":[VERSION]},
            "operation":{"type":"string","enum":[OPERATION]},
            "request_id":{"type":"string","pattern":"^req_[0-9a-f]{32}$"},
            "kind":{"type":"string","enum":[self.kind]}
        }});
        let required = schema["required"].as_array_mut().unwrap();
        if let Some(code) = self.code {
            required.extend([json!("code"), json!("message")]);
            schema["properties"]["code"] = json!({"type":"string","enum":[code]});
            schema["properties"]["message"] = json!({"type":"string"});
        } else {
            required.push(json!("data"));
            schema["properties"]["data"] = json!({"$ref":"#/components/schemas/SuccessData"});
        }
        schema
    }
}

fn reply(id: &RequestId, result: Result<Acknowledged, ActionError>) -> Response {
    match result {
        Ok(ack) => success().render(id, Some(project(ack))),
        Err(ActionError::Rejected(r)) => rejection(r).render(id, None),
        Err(ActionError::Failed {
            primary:
                StopReason::Execution {
                    kind: FailureKind::Busy,
                    ..
                },
            ..
        }) => shared(Shared::Unavailable).render(id, None),
        Err(ActionError::Failed { .. }) => shared(Shared::Internal).render(id, None),
    }
}

#[utoipa::path(post, path = "/api/memberships/role", request_body = ChangeRoleRequest, security(("BrowserSession" = [])))]
async fn endpoint(
    State(state): State<AppState>,
    Extension(id): Extension<RequestId>,
    actor: Result<Actor, crate::ApiError>,
    body: Result<Json<ChangeRoleRequest>, JsonRejection>,
) -> Response {
    let Ok(actor) = actor else {
        return shared(Shared::Unauthenticated).render(&id, None);
    };
    let Ok(Json(body)) = body else {
        return shared(Shared::Invalid).render(&id, None);
    };
    let (Ok(project_id), Ok(user_id)) = (
        crate::invitations::id(&body.project_id),
        crate::invitations::id(&body.user_id),
    ) else {
        return shared(Shared::Invalid).render(&id, None);
    };
    let input = action::ChangeRole {
        project_id,
        user_id,
        role: match body.role {
            Role::Owner => iris_sqlite_spike::MemberRole::Owner,
            Role::Editor => iris_sqlite_spike::MemberRole::Editor,
            Role::Viewer => iris_sqlite_spike::MemberRole::Viewer,
        },
    };
    let mut conn = match iris_sqlite_spike::connect(&state.database).await {
        Ok(conn) => conn,
        Err(e) => {
            return shared(if iris_sqlite_spike::is_busy(&e) {
                Shared::Unavailable
            } else {
                Shared::Internal
            })
            .render(&id, None);
        }
    };
    reply(&id, action::change_role(&mut conn, &actor, input).await)
}

/// Explicit ecosystem registration; assembly/export runs independently of docs serving.
pub fn routes() -> (Router<AppState>, utoipa::openapi::OpenApi) {
    let (router, mut api) = utoipa_axum::router::OpenApiRouter::new()
        .routes(utoipa_axum::routes!(endpoint))
        .split_for_parts();
    bridge(&mut api);
    (router, api)
}

fn bridge(api: &mut utoipa::openapi::OpenApi) {
    use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
    use utoipa::{PartialSchema, ToSchema};
    api.info.title = "Iris isolated S16 experiment".into();
    api.info.version = "1".into();
    api.info.license = None;
    assert_eq!(
        api.paths.paths.len(),
        1,
        "S16 requires exactly one collected path"
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
        "S16 requires exactly one operation"
    );
    let operation = item.post.as_mut().expect("S16 requires a collected POST");
    assert_eq!(
        operation.operation_id.as_deref(),
        Some("endpoint"),
        "S16 unexpected collected handler identity"
    );
    operation.operation_id = Some(PUBLIC_ID.into());
    let mut groups = std::collections::BTreeMap::<String, Vec<Value>>::new();
    let mappings = std::iter::once(success())
        .chain(Rejection::VARIANTS.iter().copied().map(rejection))
        .chain(Shared::VARIANTS.iter().copied().map(shared));
    let mut codes = std::collections::HashSet::new();
    for mapping in mappings {
        if let Some(code) = mapping.code {
            assert!(codes.insert(code), "duplicate public code");
        }
        groups
            .entry(mapping.status.to_string())
            .or_default()
            .push(mapping.schema());
    }
    operation.responses = serde_json::from_value(json!(groups.into_iter().map(|(status, branches)| (status, json!({"description":"S16 response", "content":{"application/json":{"schema":{"oneOf":branches}}}}))).collect::<std::collections::BTreeMap<_,_>>())).unwrap();
    operation.extensions = Some(serde_json::from_value(json!({"x-iris": {
        "operation":OPERATION, "schema_version":VERSION,
        "recovery":{"inspect":false,"read":false,"replay":false,"new_submission":"current authority and intent required"},
        "prerequisites":Rejection::VARIANTS.iter().filter_map(|r| r.descriptor().prerequisite.map(|p| (r.descriptor().code,p))).collect::<std::collections::BTreeMap<_,_>>()
    }})).unwrap());
    let components = api.components.get_or_insert_with(Default::default);
    components.add_security_scheme("BrowserSession", SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
        "__Host-iris-session", "Same-origin session and X-Iris-Csrf required. Explicit HTTP-loopback test mode uses iris-session-dev."
    ))));
    components
        .schemas
        .insert("SuccessData".into(), SuccessData::schema());
    let mut dependencies = Vec::new();
    SuccessData::schemas(&mut dependencies);
    components.schemas.extend(dependencies);
}

/// Apply only to this collected one-operation router. The method check keeps
/// method-not-allowed outside this operation's contract.
pub fn authenticated(auth: Auth) -> Router<AppState> {
    boundary(
        auth.layer(routes().0)
            .fallback(|| async { StatusCode::NOT_FOUND }),
    )
}

fn boundary(router: Router<AppState>) -> Router<AppState> {
    router.route_layer(middleware::from_fn(
        async |mut request: Request, next: Next| {
            if request.method() != Method::POST {
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
                Some(ErrorCode::Unavailable) => Some(Shared::Unavailable),
                _ => None,
            };
            if let Some(mapping) = mapping {
                let mut rendered = shared(mapping).render(&id, None);
                // Preserve cookies/no-store, not the old body's length or MIME type.
                for (name, value) in response.headers() {
                    if name != axum::http::header::CONTENT_TYPE
                        && name != axum::http::header::CONTENT_LENGTH
                    {
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

#[cfg(test)]
mod tests;
