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

        let api_state = ApiState::from_ref(state);

        let claims = api_state
            .decode_jwt(token)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token").into_response())?;

        let revoked = api_state.is_jwt_revoked(&claims.jti).await.map_err(|e| {
            tracing::error!("Redis error checking JWT revocation: {e:#}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
        })?;

        if revoked {
            tracing::warn!(sub = %claims.sub, jti = %claims.jti, "rejected revoked JWT");
            return Err((StatusCode::UNAUTHORIZED, "Token has been revoked").into_response());
        }

        Ok(AuthUser(claims))
    }
}

// ── Google ────────────────────────────────────────────────────────────────────

pub async fn login(State(state): State<ApiState>) -> Response {
    login_impl(state).await.unwrap_or_else(|e| {
        tracing::error!("OAuth login error: {e:#}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Authentication failed").into_response()
    })
}

pub async fn logout(State(state): State<ApiState>, AuthUser(claims): AuthUser) -> Response {
    logout_impl(state, claims).await.unwrap_or_else(|e| {
        tracing::error!("Logout error: {e:#}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Logout failed").into_response()
    })
}

pub async fn callback(
    State(state): State<ApiState>,
    Query(params): Query<OAuthCallbackQuery>,
) -> Response {
    callback_impl(state, params).await.unwrap_or_else(|e| {
        tracing::error!("OAuth callback error: {e:#}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Authentication failed").into_response()
    })
}

// ── Impl ────────────────────────────────────────────────────────────────────

pub async fn login_impl(state: ApiState) -> anyhow::Result<Response> {
    let csrf_state = Uuid::new_v4().to_string();
    state.store_oauth_state(&csrf_state).await?;

    tracing::info!("OAuth login initiated");

    let secrets = state.secrets();
    let redirect_uri = format!("{}/auth/callback", state.api_config().oauth.base_url);

    let mut auth_url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth")?;

    auth_url
        .query_pairs_mut()
        .append_pair("client_id", &secrets.client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid email profile")
        .append_pair("state", &csrf_state);

    Ok(Redirect::to(auth_url.as_str()).into_response())
}

pub async fn logout_impl(state: ApiState, claims: Claims) -> anyhow::Result<Response> {
    state.revoke_jwt(&claims).await?;
    tracing::info!(sub = %claims.sub, email = %claims.email, jti = %claims.jti, "user logged out");
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn callback_impl(state: ApiState, params: OAuthCallbackQuery) -> anyhow::Result<Response> {
    if !state.consume_oauth_state(&params.state).await? {
        tracing::warn!("OAuth callback rejected: invalid CSRF state");
        return Ok((StatusCode::BAD_REQUEST, "Invalid state parameter").into_response());
    }

    let secrets = state.secrets();
    let redirect_uri = format!("{}/auth/callback", state.api_config().oauth.base_url);

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

    let db_user_id = state
        .sqlx_client()
        .upsert_user(&user.sub, &user.email, &user.name)
        .await?;

    tracing::info!(user_id = %db_user_id, email = %user.email, "user authenticated");

    let jwt = state.issue_jwt(&db_user_id.to_string(), &user.email, &user.name)?;
    Ok(Json(AuthResponse { token: jwt }).into_response())
}
