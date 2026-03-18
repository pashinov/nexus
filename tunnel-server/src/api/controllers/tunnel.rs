use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use futures_util::StreamExt;
use nexus_utils::tunnel::{Frame, Headers};
use serde::Serialize;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::api::controllers::auth::AuthUser;
use crate::registry::{DeviceSession, ResponseHead, StreamRegistration};
use crate::state::TunnelState;

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub url: String,
}

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
    if let Err(err) = state
        .redis()
        .store_session(&session_token, &device_id.to_string(), ttl)
        .await
    {
        tracing::error!("failed to store session: {err}");
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

pub async fn proxy(State(state): State<TunnelState>, req: Request) -> Response {
    let token = match extract_token(req.headers(), &state.api_config().tunnel_domain) {
        Some(token) => token,
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
        Err(err) => {
            tracing::error!("redis error: {err}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    let Some(session) = state.registry().get(device_id) else {
        return (StatusCode::SERVICE_UNAVAILABLE, "device not connected").into_response();
    };

    let stream_id = Uuid::new_v4();
    let registration =
        match session.register_stream(stream_id, state.api_config().stream_channel_capacity) {
            Ok(registration) => registration,
            Err(err) => {
                tracing::warn!(%device_id, "failed to register stream: {err:#}");
                return (StatusCode::SERVICE_UNAVAILABLE, "device is overloaded").into_response();
            }
        };

    let (parts, body) = req.into_parts();
    let content_length = parts
        .headers
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok()?.parse().ok());

    let open_frame = Frame::OpenStream {
        stream_id,
        method: parts.method,
        path_and_query: parts
            .uri
            .path_and_query()
            .cloned()
            .unwrap_or_else(|| "/".parse().expect("root path_and_query")),
        headers: sanitized_headers(parts.headers),
        content_length,
    };

    if let Err(err) = session.send_frame(open_frame).await {
        tracing::warn!(%device_id, %stream_id, "failed to send stream open: {err:#}");
        session.cancel_stream(stream_id).await;
        return (StatusCode::SERVICE_UNAVAILABLE, "device not connected").into_response();
    }

    let session_for_body = session.clone();
    let max_chunk_size = state.api_config().max_chunk_size_bytes;
    tokio::spawn(async move {
        if let Err(err) =
            forward_request_body(&session_for_body, stream_id, body, max_chunk_size).await
        {
            tracing::warn!(%stream_id, "request body forwarding failed: {err:#}");
            session_for_body.cancel_stream(stream_id).await;
        }
    });

    build_streaming_response(state, session, stream_id, registration).await
}

async fn build_streaming_response(
    state: TunnelState,
    session: Arc<DeviceSession>,
    stream_id: Uuid,
    registration: StreamRegistration,
) -> Response {
    let head = match tokio::time::timeout(
        Duration::from_secs(state.api_config().response_head_timeout_secs),
        registration.head_rx,
    )
    .await
    {
        Ok(Ok(head)) => head,
        Ok(Err(_)) => {
            session.cancel_stream(stream_id).await;
            return (StatusCode::BAD_GATEWAY, "device closed stream").into_response();
        }
        Err(_) => {
            session.cancel_stream(stream_id).await;
            return (StatusCode::GATEWAY_TIMEOUT, "device response timeout").into_response();
        }
    };

    response_from_stream(head, registration.body_rx, session, stream_id)
}

fn response_from_stream(
    head: ResponseHead,
    body_rx: tokio::sync::mpsc::Receiver<Result<Bytes, std::io::Error>>,
    session: Arc<DeviceSession>,
    stream_id: Uuid,
) -> Response {
    let status = StatusCode::from_u16(head.status).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut headers = HeaderMap::new();
    headers.extend(head.headers);

    let body_stream = CancelOnDropStream {
        inner: ReceiverStream::new(body_rx),
        finished: false,
        session,
        stream_id,
    };

    (status, headers, Body::from_stream(body_stream)).into_response()
}

async fn forward_request_body(
    session: &DeviceSession,
    stream_id: Uuid,
    body: Body,
    max_chunk_size: usize,
) -> anyhow::Result<()> {
    let mut stream = body.into_data_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        for slice in chunk.chunks(max_chunk_size) {
            session
                .send_frame(Frame::RequestBodyChunk {
                    stream_id,
                    data: Bytes::copy_from_slice(slice),
                })
                .await?;
        }
    }

    session
        .send_frame(Frame::RequestBodyEnd { stream_id })
        .await?;

    Ok(())
}

fn extract_token(headers: &HeaderMap, tunnel_domain: &str) -> Option<String> {
    let host = headers.get("host")?.to_str().ok()?;
    let suffix = format!(".{tunnel_domain}");
    host.strip_suffix(&suffix).map(|value| value.to_owned())
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

fn sanitized_headers(headers: HeaderMap) -> Headers {
    let mut sanitized = HeaderMap::new();
    for (name, value) in &headers {
        if is_hop_by_hop(name.as_str()) {
            continue;
        }
        sanitized.append(name.clone(), value.clone());
    }
    sanitized
}

struct CancelOnDropStream<S> {
    inner: S,
    finished: bool,
    session: Arc<DeviceSession>,
    stream_id: Uuid,
}

impl<S> futures_util::Stream for CancelOnDropStream<S>
where
    S: futures_util::Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(None) => {
                self.finished = true;
                Poll::Ready(None)
            }
            other => other,
        }
    }
}

impl<S> Drop for CancelOnDropStream<S> {
    fn drop(&mut self) {
        if self.finished {
            return;
        }

        let stream_id = self.stream_id;
        let session = self.session.clone();
        tokio::spawn(async move {
            session.cancel_stream(stream_id).await;
        });
    }
}
