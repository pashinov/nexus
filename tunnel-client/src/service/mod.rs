use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::config::AppConfig;

// ── Protocol types (mirror of tunnel-server proto) ────────────────────────

#[derive(Debug, Deserialize)]
struct TunnelRequest {
    id: Uuid,
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}

#[derive(Debug, Serialize)]
struct TunnelResponse {
    id: Uuid,
    status: u16,
    headers: HashMap<String, String>,
    body: Option<String>,
}

// ── Service entry point ───────────────────────────────────────────────────

pub async fn tunnel_service(config: AppConfig, token: CancellationToken) -> Result<()> {
    let cfg = config.tunnel;
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .build()?;

    let mut wait = cfg.reconnect_base_secs;

    loop {
        if token.is_cancelled() {
            break;
        }

        let url = format!(
            "{}/device/connect?device_id={}",
            cfg.server_url.trim_end_matches('/'),
            cfg.device_id
        );

        tracing::info!(%url, "connecting to tunnel-server");

        match connect_async(&url).await {
            Ok((ws, _)) => {
                wait = cfg.reconnect_base_secs; // reset backoff on successful connect
                tracing::info!("connected");

                if let Err(e) = run_session(ws, &cfg.local_url, &http, &token).await {
                    tracing::warn!("session ended: {e:#}");
                } else {
                    tracing::info!("session closed cleanly");
                }
            }
            Err(e) => {
                tracing::warn!(wait, "failed to connect: {e:#}");
            }
        }

        if token.is_cancelled() {
            break;
        }

        tracing::info!(wait, "reconnecting in {wait}s...");
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(wait)) => {}
            _ = token.cancelled() => break,
        }

        wait = (wait * 2).min(cfg.reconnect_max_secs);
    }

    tracing::info!("tunnel-client stopped");
    Ok(())
}

// ── Per-connection session loop ───────────────────────────────────────────

async fn run_session(
    ws: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    local_url: &str,
    http: &reqwest::Client,
    token: &CancellationToken,
) -> Result<()> {
    let (mut sink, mut stream) = ws.split();

    // Channel for sending serialised responses back through the WS sink.
    let (resp_tx, mut resp_rx) = mpsc::channel::<String>(64);

    // Spawn the sink-writer task.
    tokio::spawn(async move {
        while let Some(msg) = resp_rx.recv().await {
            if sink.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    });

    loop {
        tokio::select! {
            _ = token.cancelled() => return Ok(()),

            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<TunnelRequest>(&text) {
                            Ok(req) => {
                                let http = http.clone();
                                let local_url = local_url.to_owned();
                                let tx = resp_tx.clone();
                                tokio::spawn(async move {
                                    let resp = handle_request(&http, &local_url, req).await;
                                    if let Ok(json) = serde_json::to_string(&resp) {
                                        let _ = tx.send(json).await;
                                    }
                                });
                            }
                            Err(e) => tracing::warn!("failed to parse request: {e}"),
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        // tungstenite handles pong automatically, but we log it.
                        tracing::debug!("ping ({} bytes)", payload.len());
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        return Ok(());
                    }
                    Some(Err(e)) => {
                        return Err(e.into());
                    }
                    _ => {}
                }
            }
        }
    }
}

// ── Forward one request to the local HTTP service ────────────────────────

async fn handle_request(
    http: &reqwest::Client,
    local_url: &str,
    req: TunnelRequest,
) -> TunnelResponse {
    let url = format!("{}{}", local_url.trim_end_matches('/'), req.path);

    tracing::debug!(id = %req.id, method = %req.method, path = %req.path, "proxying");

    let method = match reqwest::Method::from_bytes(req.method.as_bytes()) {
        Ok(m) => m,
        Err(_) => {
            return error_response(req.id, 400, "invalid method");
        }
    };

    let mut builder = http.request(method, &url);

    for (k, v) in &req.headers {
        // Skip hop-by-hop headers that don't make sense when re-sending locally.
        if matches!(
            k.to_ascii_lowercase().as_str(),
            "host" | "connection" | "transfer-encoding" | "upgrade" | "accept-encoding"
        ) {
            continue;
        }
        builder = builder.header(k, v);
    }

    if let Some(b64) = req.body {
        match base64::engine::general_purpose::STANDARD.decode(b64) {
            Ok(bytes) => builder = builder.body(bytes),
            Err(_) => return error_response(req.id, 400, "invalid body encoding"),
        }
    }

    match builder.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let headers: HashMap<String, String> = resp
                .headers()
                .iter()
                .filter(|(k, _)| {
                    !matches!(
                        k.as_str(),
                        "connection" | "keep-alive" | "transfer-encoding" | "content-length"
                    )
                })
                .filter_map(|(k, v)| {
                    v.to_str()
                        .ok()
                        .map(|val| (k.as_str().to_owned(), val.to_owned()))
                })
                .collect();

            let body = match resp.bytes().await {
                Ok(b) if !b.is_empty() => {
                    Some(base64::engine::general_purpose::STANDARD.encode(&b))
                }
                Ok(_) => None,
                Err(e) => {
                    tracing::warn!(id = %req.id, "failed to read response body: {e:#}");
                    None
                }
            };

            TunnelResponse {
                id: req.id,

                status,
                headers,
                body,
            }
        }
        Err(e) => {
            tracing::warn!(id = %req.id, "local request failed: {e:#}");
            error_response(req.id, 502, "local service unavailable")
        }
    }
}

fn error_response(id: Uuid, status: u16, msg: &str) -> TunnelResponse {
    let body = base64::engine::general_purpose::STANDARD.encode(msg.as_bytes());
    TunnelResponse {
        id,
        status,
        headers: HashMap::from([("content-type".to_owned(), "text/plain".to_owned())]),
        body: Some(body),
    }
}
