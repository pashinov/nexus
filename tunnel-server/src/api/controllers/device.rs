use anyhow::Result;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use nexus_utils::tunnel::{Frame, decode_frame, encode_frame};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::registry::DeviceSession;
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

async fn handle_device_socket(socket: WebSocket, device_id: Uuid, state: TunnelState) {
    let (mut sink, mut stream) = socket.split();

    let (frame_tx, mut frame_rx) =
        mpsc::channel::<Frame>(state.api_config().stream_channel_capacity);

    let shutdown = state.shutdown_token().child_token();

    let (session, previous) = state.registry().register(
        device_id,
        state.api_config().max_concurrent_streams_per_device,
        frame_tx,
        shutdown.clone(),
    );

    if let Some(previous) = previous {
        tracing::warn!(%device_id, "replacing existing device session");
        previous.shutdown();
    }

    tracing::info!(%device_id, "device connected");

    let handle = tokio::spawn({
        let shutdown = shutdown.clone();

        async move {
            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    frame = frame_rx.recv() => {
                        let Some(frame) = frame else {
                            break;
                        };

                        let payload = encode_frame(&frame)?;
                        tokio::select! {
                            _ = shutdown.cancelled() => break,
                            result = sink.send(Message::Binary(payload.into())) => { result?; }
                        }
                    }
                }
            }

            sink.close().await?;

            Ok::<_, anyhow::Error>(())
        }
    });

    if let Err(err) = device_reader_loop(&session, &mut stream).await {
        tracing::error!(%device_id, "device session ended: {err:#}");
    }

    shutdown.cancel();

    match handle.await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => tracing::error!(%device_id, "device writer failed: {err:#}"),
        Err(err) if err.is_cancelled() => {}
        Err(err) => std::panic::resume_unwind(err.into_panic()),
    }

    session.close_all("device disconnected").await;

    state.registry().unregister(device_id, &session);

    tracing::info!(%device_id, "device disconnected");
}

async fn device_reader_loop(
    session: &Arc<DeviceSession>,
    stream: &mut futures_util::stream::SplitStream<WebSocket>,
) -> Result<()> {
    let shutdown = session.shutdown_token();

    loop {
        let msg = tokio::select! {
            _ = shutdown.cancelled() => return Ok(()),
            msg = stream.next() => msg,
        };

        match msg {
            Some(Ok(Message::Binary(payload))) => {
                let frame = decode_frame(&payload)?;
                session.deliver_frame(frame).await?;
            }
            Some(Ok(Message::Close(_))) | None => break,
            Some(Ok(Message::Pong(_) | Message::Ping(_))) => {}
            Some(Err(err)) => return Err(err.into()),
            Some(Ok(Message::Text(_))) => return Err(anyhow::anyhow!("unexpected text frame")),
        }
    }

    Ok(())
}
