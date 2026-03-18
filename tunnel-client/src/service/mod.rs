use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use anyhow::{Result, anyhow};
use bytes::Bytes;
use dashmap::DashMap;
use futures_util::{SinkExt, Stream, StreamExt};
use nexus_utils::tunnel::{Frame, Headers, decode_frame, encode_frame};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::config::{AppConfig, TunnelConfig};

const WS_PING_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

pub async fn tunnel_service(config: AppConfig, token: CancellationToken) -> Result<()> {
    let cfg = config.tunnel;
    let http = reqwest::Client::builder().build()?;

    loop {
        if token.is_cancelled() {
            break;
        }

        let url = format!(
            "{}/device/connect?device_id={}",
            cfg.server_url.trim_end_matches('/'),
            cfg.device_id
        );

        tracing::info!(server_url = %cfg.server_url, "tunnel-server connecting");

        match connect_async(&url).await {
            Ok((ws, _)) => {
                tracing::info!("tunnel-server connected");

                if let Err(err) = run_session(ws, cfg.clone(), http.clone(), token.clone()).await {
                    tracing::error!("tunnel-server connection ended with error: {err:#}");
                }
            }
            Err(err) => {
                tracing::warn!("tunnel-server connection failed: {err:#}");
            }
        }

        if token.is_cancelled() {
            break;
        }

        tokio::select! {
            _ = tokio::time::sleep(cfg.reconnect_timeout) => {}
            _ = token.cancelled() => break,
        }
    }

    tracing::info!("tunnel-client stopped");

    Ok(())
}

async fn run_session(
    ws: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    cfg: TunnelConfig,
    http: reqwest::Client,
    token: CancellationToken,
) -> Result<()> {
    let (mut sink, mut stream) = ws.split();

    let (frame_tx, mut frame_rx) = mpsc::channel::<Frame>(cfg.frame_channel_capacity);

    let max_concurrent_streams = cfg.max_concurrent_streams;

    let session = Arc::new(ClientSession {
        cfg,
        http,
        frame_tx,
        streams: DashMap::new(),
        permits: Arc::new(Semaphore::new(max_concurrent_streams)),
    });

    let shutdown = CancellationToken::new();

    let handle = tokio::spawn({
        let shutdown = shutdown.clone();
        async move {
            let mut ping = tokio::time::interval(WS_PING_INTERVAL);
            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = ping.tick() => {
                        tokio::select! {
                            _ = shutdown.cancelled() => break,
                            result = sink.send(Message::Ping(Bytes::new())) => { result?; }
                        }
                    }
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

            Ok::<(), anyhow::Error>(())
        }
    });

    if let Err(err) = session.reader_loop(&mut stream, &token).await {
        tracing::error!("session reader ended: {err:#}");
    }

    session.close_all();

    shutdown.cancel();

    match handle.await {
        Ok(result) => result?,
        Err(err) if err.is_cancelled() => {}
        Err(err) => std::panic::resume_unwind(err.into_panic()),
    }

    Ok(())
}

struct ClientSession {
    cfg: TunnelConfig,
    http: reqwest::Client,
    frame_tx: mpsc::Sender<Frame>,
    streams: DashMap<Uuid, StreamState>,
    permits: Arc<Semaphore>,
}

struct StreamState {
    request_tx: Option<mpsc::Sender<Result<Bytes, io::Error>>>,
    cancel: CancellationToken,
}

impl ClientSession {
    async fn reader_loop(
        self: &Arc<Self>,
        stream: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
        token: &CancellationToken,
    ) -> Result<()> {
        loop {
            tokio::select! {
                _ = token.cancelled() => return Ok(()),
                msg = stream.next() => match msg {
                    Some(Ok(Message::Binary(payload))) => {
                    let frame = decode_frame(&payload)?;
                    self.handle_frame(frame).await?;
                }
                    Some(Ok(Message::Ping(_))) => {}
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Frame(_))) => {}
                    Some(Ok(Message::Close(_))) | None => return Ok(()),
                    Some(Ok(Message::Text(_))) => return Err(anyhow!("unexpected text frame")),
                    Some(Err(err)) => return Err(err.into()),
                }
            }
        }
    }

    async fn handle_frame(self: &Arc<Self>, frame: Frame) -> Result<()> {
        match frame {
            Frame::OpenStream {
                stream_id,
                method,
                path_and_query,
                headers,
                ..
            } => {
                self.open_stream(stream_id, method, path_and_query, headers)
                    .await
            }
            Frame::RequestBodyChunk { stream_id, data } => {
                let sender: mpsc::Sender<Result<Bytes, io::Error>> = match self
                    .streams
                    .get(&stream_id)
                    .and_then(|entry| entry.request_tx.clone())
                {
                    Some(sender) => sender,
                    None => {
                        self.send_error(stream_id, 404, "stream not found").await?;
                        return Ok(());
                    }
                };

                if sender.send(Ok(data)).await.is_err() {
                    self.cancel_stream(stream_id).await?;
                }

                Ok(())
            }
            Frame::RequestBodyEnd { stream_id } => {
                if let Some(mut entry) = self.streams.get_mut(&stream_id) {
                    entry.request_tx.take();
                }
                Ok(())
            }
            Frame::CancelStream { stream_id } => self.cancel_stream(stream_id).await,
            Frame::ResponseHead { .. }
            | Frame::ResponseBodyChunk { .. }
            | Frame::ResponseBodyEnd { .. }
            | Frame::ErrorStream { .. } => Err(anyhow!("unexpected frame from server")),
        }
    }

    async fn open_stream(
        self: &Arc<Self>,
        stream_id: Uuid,
        method: reqwest::Method,
        path_and_query: http::uri::PathAndQuery,
        headers: Headers,
    ) -> Result<()> {
        let permit = match self.permits.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                self.send_error(stream_id, 503, "too many active streams")
                    .await?;
                return Ok(());
            }
        };

        let (request_tx, request_rx) = mpsc::channel::<Result<Bytes, io::Error>>(16);
        let cancel = CancellationToken::new();
        self.streams.insert(
            stream_id,
            StreamState {
                request_tx: Some(request_tx),
                cancel: cancel.clone(),
            },
        );

        let local_url = format!(
            "{}{}",
            self.cfg.local_url.trim_end_matches('/'),
            path_and_query.as_str()
        );
        let mut builder = self.http.request(method, &local_url);
        for (name, value) in &headers {
            if is_hop_by_hop(name.as_str()) {
                continue;
            }
            builder = builder.header(name, value);
        }

        let body = reqwest::Body::wrap_stream(RequestBodyStream { rx: request_rx });
        let builder = builder.body(body);

        let session = self.clone();
        tokio::spawn(async move {
            session
                .run_local_request(stream_id, builder, cancel, permit)
                .await;
        });

        Ok(())
    }

    async fn run_local_request(
        self: Arc<Self>,
        stream_id: Uuid,
        builder: reqwest::RequestBuilder,
        cancel: CancellationToken,
        permit: OwnedSemaphorePermit,
    ) {
        let _permit = permit;

        let response = tokio::select! {
            _ = cancel.cancelled() => {
                self.streams.remove(&stream_id);
                return;
            }
            response = builder.send() => response,
        };

        let response = match response {
            Ok(response) => response,
            Err(err) => {
                self.streams.remove(&stream_id);
                let _ = self
                    .send_error(stream_id, 502, &format!("local request failed: {err}"))
                    .await;
                return;
            }
        };

        let headers = response_headers(response.headers());
        if self
            .send_frame(Frame::ResponseHead {
                stream_id,
                status: response.status().as_u16(),
                headers,
            })
            .await
            .is_err()
        {
            self.streams.remove(&stream_id);
            return;
        }

        let mut body = response.bytes_stream();
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    self.streams.remove(&stream_id);
                    return;
                }
                chunk = body.next() => {
                    match chunk {
                        Some(Ok(chunk)) => {
                            if self.send_frame(Frame::ResponseBodyChunk {
                                stream_id,
                                data: chunk,
                            }).await.is_err() {
                                self.streams.remove(&stream_id);
                                return;
                            }
                        }
                        Some(Err(err)) => {
                            self.streams.remove(&stream_id);
                            let _ = self.send_error(stream_id, 502, &format!("local response stream failed: {err}")).await;
                            return;
                        }
                        None => {
                            self.streams.remove(&stream_id);
                            let _ = self.send_frame(Frame::ResponseBodyEnd { stream_id }).await;
                            return;
                        }
                    }
                }
            }
        }
    }

    async fn cancel_stream(self: &Arc<Self>, stream_id: Uuid) -> Result<()> {
        if let Some((_, entry)) = self.streams.remove(&stream_id) {
            entry.cancel.cancel();
        }
        Ok(())
    }

    async fn send_frame(&self, frame: Frame) -> Result<()> {
        self.frame_tx
            .send(frame)
            .await
            .map_err(|err| anyhow!("frame queue closed: {err}"))
    }

    async fn send_error(&self, stream_id: Uuid, status: u16, message: &str) -> Result<()> {
        self.send_frame(Frame::ErrorStream {
            stream_id,
            status,
            message: message.to_owned(),
        })
        .await
    }

    fn close_all(&self) {
        let stream_ids: Vec<Uuid> = self.streams.iter().map(|entry| *entry.key()).collect();
        for stream_id in stream_ids {
            if let Some((_, entry)) = self.streams.remove(&stream_id) {
                entry.cancel.cancel();
            }
        }
    }
}

struct RequestBodyStream {
    rx: mpsc::Receiver<Result<Bytes, io::Error>>,
}

impl Stream for RequestBodyStream {
    type Item = Result<Bytes, io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_recv(cx)
    }
}

fn response_headers(headers: &reqwest::header::HeaderMap) -> Headers {
    let mut filtered = Headers::new();
    for (name, value) in headers {
        if is_hop_by_hop(name.as_str()) {
            continue;
        }
        filtered.append(name.clone(), value.clone());
    }
    filtered
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
