use axum::Json;
use axum::extract::{FromRef, FromRequestParts, Query, State};
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Redirect, Response};
use url::Url;
use uuid::Uuid;

use crate::api::models::auth::{
    AuthResponse, Claims, OAuthCallbackQuery, OAuthTokenResponse, UserInfo,
};
use crate::api::state::ApiState;

// ── Auth extractor ─────────────────────────────────────────────────────────────

/// Authenticated user extracted from a Bearer JWT token.
pub struct AuthUser(pub Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    ApiState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| {
                (StatusCode::UNAUTHORIZED, "Missing authorization header").into_response()
            })?;

        ApiState::from_ref(state)
            .decode_jwt(token)
            .map(AuthUser)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token").into_response())
    }
}

// ── Google ────────────────────────────────────────────────────────────────────

pub async fn login(State(state): State<ApiState>) -> impl IntoResponse {
    let csrf_state = Uuid::new_v4().to_string();
    state.store_oauth_state(&csrf_state);

    let secrets = state.secrets();
    let redirect_uri = format!("{}/auth/callback", state.config().oauth.base_url);

    let mut auth_url =
        Url::parse("https://accounts.google.com/o/oauth2/v2/auth").expect("shouldn't happen");

    auth_url
        .query_pairs_mut()
        .append_pair("client_id", &secrets.client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid email profile")
        .append_pair("state", &csrf_state);

    Redirect::to(auth_url.as_str())
}

pub async fn callback(
    State(state): State<ApiState>,
    Query(params): Query<OAuthCallbackQuery>,
) -> Response {
    callback_inner(state, params).await.unwrap_or_else(|e| {
        tracing::error!("Google OAuth error: {e:#}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Authentication failed").into_response()
    })
}

async fn callback_inner(state: ApiState, params: OAuthCallbackQuery) -> anyhow::Result<Response> {
    if !state.consume_oauth_state(&params.state) {
        return Ok((StatusCode::BAD_REQUEST, "Invalid state parameter").into_response());
    }

    let secrets = state.secrets();
    let redirect_uri = format!("{}/auth/callback", state.config().oauth.base_url);

    // Exchange authorization code for access token
    let token: OAuthTokenResponse = state
        .http_client()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", params.code.as_str()),
            ("client_id", secrets.client_id.as_str()),
            ("client_secret", secrets.client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // Fetch user info from Google
    let user: UserInfo = state
        .http_client()
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(&token.access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // TODO: find or create user in DB
    // let db_user = db.upsert_user(&user.sub, &user.email, &user.name, "google").await?;

    let jwt = state.issue_jwt(&user.sub, &user.email, &user.name)?;
    Ok(Json(AuthResponse { token: jwt }).into_response())
}
