use std::time::Duration;

use axum::extract::{Query, State, WebSocketUpgrade};
use axum::extract::ws::{Message, WebSocket};
use axum::response::Response;
use serde::Deserialize;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::models::{TunnelRequest, TunnelResponse};
use crate::state::TunnelState;

#[derive(Debug, Deserialize)]
pub struct ConnectQuery {
    pub device_id: Uuid,
}

pub async fn connect(
    ws: WebSocketUpgrade,
    Query(query): Query<ConnectQuery>,
    State(state): State<TunnelState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_device_socket(socket, query.device_id, state))
}

async fn handle_device_socket(mut socket: WebSocket, device_id: Uuid, state: TunnelState) {
    let (tx, mut rx) = mpsc::channel::<TunnelRequest>(32);
    state.registry().register(device_id, tx);
    tracing::info!(%device_id, "device connected");

    let mut ping_interval = tokio::time::interval(Duration::from_secs(30));
    ping_interval.tick().await;
    let mut awaiting_pong = false;

    loop {
        tokio::select! {
            req = rx.recv() => {
                let Some(req) = req else { break };
                let json = match serde_json::to_string(&req) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::error!("failed to serialize request: {e}");
                        continue;
                    }
                };
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<TunnelResponse>(&text) {
                            Ok(resp) => state.registry().complete_request(resp),
                            Err(e) => tracing::warn!("failed to parse device response: {e}"),
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        awaiting_pong = false;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        tracing::warn!(%device_id, "WS error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
            _ = ping_interval.tick() => {
                if awaiting_pong {
                    tracing::warn!(%device_id, "pong timeout");
                    break;
                }
                if socket.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
                awaiting_pong = true;
            }
        }
    }

    state.registry().unregister(device_id);
    tracing::info!(%device_id, "device disconnected");
}
