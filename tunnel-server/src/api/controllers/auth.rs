use axum::extract::{FromRef, FromRequestParts};
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

use crate::state::{Claims, TunnelState};

/// Authenticated user extracted from a Bearer JWT token.
pub struct AuthUser(pub Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    TunnelState: FromRef<S>,
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
                (StatusCode::UNAUTHORIZED, "missing Authorization header").into_response()
            })?;

        let tunnel_state = TunnelState::from_ref(state);

        let claims = tunnel_state.decode_jwt(token).map_err(|e| {
            tracing::warn!("JWT validation failed: {e}");
            (StatusCode::UNAUTHORIZED, "invalid token").into_response()
        })?;

        Ok(AuthUser(claims))
    }
}
