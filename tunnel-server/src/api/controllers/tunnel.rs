use std::collections::HashMap;
use std::time::Duration;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::Engine as _;
use serde::Serialize;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::api::controllers::auth::AuthUser;
use crate::models::{TunnelRequest, TunnelResponse};
use crate::state::TunnelState;

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub url: String,
}

/// POST /tunnel/{device_id}/session
pub async fn create_session(
    AuthUser(_claims): AuthUser,
    Path(device_id): Path<Uuid>,
    State(state): State<TunnelState>,
) -> Response {
    if !state.registry().is_online(device_id) {
        return (StatusCode::SERVICE_UNAVAILABLE, "device not connected").into_response();
    }

    let session_token = Uuid::new_v4().to_string();
    let ttl = state.api_config().session_ttl;
    if let Err(e) = state
        .redis()
        .store_session(&session_token, &device_id.to_string(), ttl)
        .await
    {
        tracing::error!("failed to store session: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to create session",
        )
            .into_response();
    }

    let scheme = &state.api_config().tunnel_scheme;
    let domain = &state.api_config().tunnel_domain;
    let url = format!("{scheme}://{session_token}.{domain}/");

    Json(SessionResponse { url }).into_response()
}

/// Catch-all proxy handler — token is extracted from the Host header.
pub async fn proxy(
    State(state): State<TunnelState>,
    method: axum::http::Method,
    headers: HeaderMap,
    uri: axum::http::Uri,
    body: axum::body::Bytes,
) -> Response {
    // Extract token from subdomain: "{token}.localhost:8001" → "{token}"
    let token = match extract_token(&headers, &state.api_config().tunnel_domain) {
        Some(t) => t,
        None => return (StatusCode::BAD_REQUEST, "missing or invalid Host header").into_response(),
    };

    let device_id: Uuid = match state.redis().get_session(&token).await {
        Ok(Some(id)) => match id.parse() {
            Ok(uuid) => uuid,
            Err(_) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, "invalid session data").into_response();
            }
        },
        Ok(None) => return (StatusCode::NOT_FOUND, "session not found").into_response(),
        Err(e) => {
            tracing::error!("redis error: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    if !state.registry().is_online(device_id) {
        return (StatusCode::SERVICE_UNAVAILABLE, "device not connected").into_response();
    }

    let path = uri
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("/")
        .to_owned();

    let body_b64 = if body.is_empty() {
        None
    } else {
        Some(base64::engine::general_purpose::STANDARD.encode(&body))
    };

    let req_headers: HashMap<String, String> = headers
        .iter()
        .filter(|(k, _)| !is_hop_by_hop(k.as_str()))
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|val| (k.as_str().to_owned(), val.to_owned()))
        })
        .collect();

    let req = TunnelRequest {
        id: Uuid::new_v4(),
        method: method.to_string(),
        path,
        headers: req_headers,
        body: body_b64,
    };

    let (resp_tx, resp_rx) = oneshot::channel::<TunnelResponse>();

    let req_id = req.id;

    if let Err(e) = state.registry().send_request(device_id, req, resp_tx).await {
        tracing::warn!("failed to send request to device: {e}");
        return (StatusCode::SERVICE_UNAVAILABLE, "device not connected").into_response();
    }

    let tunnel_resp = match tokio::time::timeout(Duration::from_secs(30), resp_rx).await {
        Ok(Ok(r)) => r,
        Ok(Err(_)) => {
            state.registry().cancel_request(req_id);
            return (StatusCode::BAD_GATEWAY, "device closed connection").into_response();
        }
        Err(_) => {
            state.registry().cancel_request(req_id);
            return (StatusCode::GATEWAY_TIMEOUT, "device response timeout").into_response();
        }
    };

    let status =
        StatusCode::from_u16(tunnel_resp.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let mut headers = HeaderMap::new();
    for (k, v) in &tunnel_resp.headers {
        if is_hop_by_hop(k) || k.eq_ignore_ascii_case("content-length") {
            continue;
        }
        if let (Ok(name), Ok(value)) = (
            axum::http::HeaderName::from_bytes(k.as_bytes()),
            axum::http::HeaderValue::from_str(v),
        ) {
            headers.insert(name, value);
        }
    }

    let body = match tunnel_resp.body {
        Some(b64) => base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap_or_default(),
        None => vec![],
    };

    (status, headers, body).into_response()
}

/// Extract the session token from the Host header subdomain.
/// Host: "abc123.localhost:8001", domain: "localhost:8001" → Some("abc123")
fn extract_token(headers: &HeaderMap, tunnel_domain: &str) -> Option<String> {
    let host = headers.get("host")?.to_str().ok()?;
    let suffix = format!(".{tunnel_domain}");
    host.strip_suffix(&suffix).map(|s| s.to_owned())
}

fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "host"
    )
}
