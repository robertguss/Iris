pub mod store;

use crate::{Actor, ApiError, AppState, ErrorCode};
use axum::{
    Extension, Json, Router,
    extract::{Query, Request},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
};
use openidconnect::OAuth2TokenResponse;
use openidconnect::{
    AccessTokenHash, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, TokenResponse,
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use store::Store;
use subtle::ConstantTimeEq;
use tower_sessions::{Expiry, Session, SessionManagerLayer, cookie::SameSite};

#[derive(Clone)]
pub struct Auth {
    pub store: Store,
    pub origin: String,
    pub issuer: String,
    pub client_id: String,
    metadata: CoreProviderMetadata,
    http: reqwest::Client,
    secret: Option<ClientSecret>,
    secure: bool,
}

fn internal(_: impl std::fmt::Display) -> ApiError {
    ApiError(ErrorCode::Internal)
}
fn rejected(_: impl std::fmt::Display) -> ApiError {
    ApiError(ErrorCode::LoginFailed)
}

impl Auth {
    pub async fn discover(
        store: Store,
        origin: String,
        issuer: String,
        client_id: String,
        secret: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let url = reqwest::Url::parse(&origin)?;
        let secure = url.scheme() == "https";
        if url.origin().ascii_serialization() != origin
            || (!secure
                && !(url.scheme() == "http"
                    && matches!(url.host_str(), Some("127.0.0.1" | "localhost"))))
        {
            return Err(
                "origin must be a canonical HTTPS origin or explicit HTTP loopback origin".into(),
            );
        }
        let issuer_url = reqwest::Url::parse(&issuer)?;
        if issuer_url.scheme() != "https"
            && !(issuer_url.scheme() == "http"
                && matches!(issuer_url.host_str(), Some("127.0.0.1" | "localhost")))
        {
            return Err("issuer must use HTTPS or explicit HTTP loopback".into());
        }
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let metadata =
            CoreProviderMetadata::discover_async(IssuerUrl::new(issuer.clone())?, &http).await?;
        Ok(Self {
            store,
            origin,
            issuer,
            client_id,
            metadata,
            http,
            secret: secret.map(ClientSecret::new),
            secure,
        })
    }

    pub fn layer(self, app: Router<AppState>) -> Router<AppState> {
        let sessions = SessionManagerLayer::new(self.store.clone())
            .with_name(if self.secure {
                "__Host-iris-session"
            } else {
                "iris-session-dev"
            })
            .with_secure(self.secure)
            .with_http_only(true)
            .with_same_site(SameSite::Lax)
            .with_expiry(Expiry::OnInactivity(time::Duration::minutes(10)));
        app.layer(middleware::from_fn(boundary))
            .layer(Extension(Arc::new(self)))
            .layer(sessions)
            .layer(middleware::from_fn(normalize))
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct BrowserSession {
    csrf: String,
    user_id: Option<i64>,
    expires_at: i64,
}

fn random() -> Result<String, ApiError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(internal)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn expiry(session: &Session, timestamp: i64) -> Result<(), ApiError> {
    session.set_expiry(Some(Expiry::AtDateTime(
        time::OffsetDateTime::from_unix_timestamp(timestamp).map_err(internal)?,
    )));
    Ok(())
}

async fn boundary(
    Extension(auth): Extension<Arc<Auth>>,
    session: Session,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let mut data = session
        .get::<BrowserSession>("browser")
        .await
        .map_err(internal)?;
    if let Some(ref value) = data {
        let user_exists = if let Some(id) = value.user_id {
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id=?)")
                .bind(id)
                .fetch_one(&auth.store.pool)
                .await
                .map_err(internal)?
        } else {
            true
        };
        if value.expires_at <= (auth.store.now)() || !user_exists {
            session.flush().await.map_err(internal)?;
            data = None;
        }
    }
    if !matches!(
        *request.method(),
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    ) {
        let token = single(request.headers(), "x-iris-csrf");
        let valid = single(request.headers(), "origin") == Some(auth.origin.as_str())
            && data
                .as_ref()
                .zip(token)
                .is_some_and(|(d, t)| bool::from(d.csrf.as_bytes().ct_eq(t.as_bytes())));
        if !valid {
            return Err(ApiError(ErrorCode::Csrf));
        }
    }
    if let Some(id) = data.and_then(|d| d.user_id) {
        request.extensions_mut().insert(Actor(id));
    }
    let response = next.run(request).await;
    if session.is_modified()
        && let Some(latest) = session
            .get::<BrowserSession>("browser")
            .await
            .map_err(internal)?
    {
        expiry(&session, latest.expires_at)?;
    }
    Ok(response)
}

fn single<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        None
    } else {
        Some(value)
    }
}

async fn normalize(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    if response.status().is_server_error() && !response.headers().contains_key(header::CONTENT_TYPE)
    {
        response = ApiError(ErrorCode::Internal).into_response();
    }
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
}

#[derive(Serialize, utoipa::ToSchema, schemars::JsonSchema)]
pub struct SessionInfo {
    pub user_id: Option<String>,
    pub csrf_token: String,
}

#[derive(Serialize, utoipa::ToSchema, schemars::JsonSchema)]
pub struct LoginInfo {
    pub authorization_url: String,
}

#[utoipa::path(get, path="/api/auth/session", responses((status=200,body=SessionInfo),(status=500,body=crate::Problem)))]
pub async fn session_info(
    Extension(auth): Extension<Arc<Auth>>,
    session: Session,
) -> Result<Json<SessionInfo>, ApiError> {
    let data = match session
        .get::<BrowserSession>("browser")
        .await
        .map_err(internal)?
    {
        Some(data) => data,
        None => {
            session.cycle_id().await.map_err(internal)?;
            let data = BrowserSession {
                csrf: random()?,
                user_id: None,
                expires_at: (auth.store.now)() + 600,
            };
            expiry(&session, data.expires_at)?;
            session.insert("browser", &data).await.map_err(internal)?;
            data
        }
    };
    Ok(Json(SessionInfo {
        user_id: data.user_id.map(|id| id.to_string()),
        csrf_token: data.csrf,
    }))
}

#[utoipa::path(post, path="/api/auth/login", responses((status=200,body=LoginInfo),(status=403,body=crate::Problem),(status=500,body=crate::Problem)), security(("BrowserSession"=[])))]
pub async fn login(
    Extension(auth): Extension<Arc<Auth>>,
    session: Session,
) -> Result<Json<LoginInfo>, ApiError> {
    let id = session.id().ok_or(ApiError(ErrorCode::Csrf))?;
    let client = CoreClient::from_provider_metadata(
        auth.metadata.clone(),
        ClientId::new(auth.client_id.clone()),
        auth.secret.clone(),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!("{}/api/auth/callback", auth.origin)).map_err(internal)?,
    );
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let (url, state, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .set_pkce_challenge(challenge)
        .url();
    sqlx::query("INSERT INTO iris_login_attempts(state,browser_id,nonce,verifier,expires_at) VALUES(?,?,?,?,?)")
        .bind(state.secret()).bind(id.to_string()).bind(nonce.secret()).bind(verifier.secret()).bind((auth.store.now)()+600)
        .execute(&auth.store.pool).await.map_err(internal)?;
    Ok(Json(LoginInfo {
        authorization_url: url.to_string(),
    }))
}

#[derive(Deserialize)]
pub struct Callback {
    state: String,
    code: Option<String>,
}

#[utoipa::path(get, path="/api/auth/callback", responses((status=303,description="Login complete"),(status=401,body=crate::Problem),(status=500,body=crate::Problem)))]
pub async fn callback(
    Extension(auth): Extension<Arc<Auth>>,
    session: Session,
    query: Result<Query<Callback>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, ApiError> {
    let Query(query) = query.map_err(rejected)?;
    let id = session.id().ok_or(ApiError(ErrorCode::LoginFailed))?;
    // Atomic consumption commits before HTTP exchange: a replay cannot exchange.
    let attempt=sqlx::query("DELETE FROM iris_login_attempts WHERE state=? AND browser_id=? AND expires_at>? RETURNING nonce,verifier")
        .bind(&query.state).bind(id.to_string()).bind((auth.store.now)()).fetch_optional(&auth.store.pool).await.map_err(internal)?
        .ok_or(ApiError(ErrorCode::LoginFailed))?;
    let client = CoreClient::from_provider_metadata(
        auth.metadata.clone(),
        ClientId::new(auth.client_id.clone()),
        auth.secret.clone(),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!("{}/api/auth/callback", auth.origin)).map_err(internal)?,
    );
    let response = client
        .exchange_code(AuthorizationCode::new(
            query.code.ok_or(ApiError(ErrorCode::LoginFailed))?,
        ))
        .map_err(rejected)?
        .set_pkce_verifier(PkceCodeVerifier::new(attempt.get("verifier")))
        .request_async(&auth.http)
        .await
        .map_err(rejected)?;
    let token = response
        .id_token()
        .ok_or(ApiError(ErrorCode::LoginFailed))?;
    let verifier = client.id_token_verifier();
    let nonce = Nonce::new(attempt.get("nonce"));
    let claims = token.claims(&verifier, &nonce).map_err(rejected)?;
    if let Some(expected) = claims.access_token_hash() {
        let actual = AccessTokenHash::from_token(
            response.access_token(),
            token.signing_alg().map_err(rejected)?,
            token.signing_key(&verifier).map_err(rejected)?,
        )
        .map_err(rejected)?;
        if actual != *expected {
            return Err(ApiError(ErrorCode::LoginFailed));
        }
    }
    let user_id = sqlx::query_scalar::<_, i64>(
        "SELECT user_id FROM iris_external_identities WHERE issuer=? AND subject=?",
    )
    .bind(&auth.issuer)
    .bind(claims.subject().as_str())
    .fetch_optional(&auth.store.pool)
    .await
    .map_err(internal)?
    .ok_or(ApiError(ErrorCode::LoginFailed))?;
    // Promotion and logout compete for the same live browser row. A callback
    // finishing token exchange after revocation must not create a new session.
    let promoted =
        sqlx::query("DELETE FROM iris_sessions WHERE id=? AND expires_at>? RETURNING id")
            .bind(id.to_string())
            .bind((auth.store.now)())
            .fetch_optional(&auth.store.pool)
            .await
            .map_err(internal)?;
    if promoted.is_none() {
        return Err(ApiError(ErrorCode::LoginFailed));
    }
    session.clear().await;
    session.cycle_id().await.map_err(internal)?;
    let data = BrowserSession {
        csrf: random()?,
        user_id: Some(user_id),
        expires_at: (auth.store.now)() + 8 * 3600,
    };
    expiry(&session, data.expires_at)?;
    session.insert("browser", data).await.map_err(internal)?;
    Ok(Redirect::to("/").into_response())
}

#[utoipa::path(post, path="/api/auth/logout", responses((status=204,description="This session logged out"),(status=403,body=crate::Problem),(status=500,body=crate::Problem)),security(("BrowserSession"=[])))]
pub async fn logout(session: Session) -> Result<StatusCode, ApiError> {
    session.flush().await.map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}
