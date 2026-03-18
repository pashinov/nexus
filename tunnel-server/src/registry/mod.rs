use std::io;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use axum::http::HeaderMap;
use bytes::Bytes;
use dashmap::DashMap;
use nexus_utils::tunnel::{Frame, Headers};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone)]
pub struct DeviceRegistry {
    devices: Arc<DashMap<Uuid, Arc<DeviceSession>>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(DashMap::new()),
        }
    }

    pub fn register(
        &self,
        device_id: Uuid,
        max_streams: usize,
        frame_tx: mpsc::Sender<Frame>,
        shutdown: CancellationToken,
    ) -> (Arc<DeviceSession>, Option<Arc<DeviceSession>>) {
        let session = Arc::new(DeviceSession::new(
            device_id,
            max_streams,
            frame_tx,
            shutdown,
        ));
        let previous = self.devices.insert(device_id, session.clone());
        (session, previous)
    }

    pub fn unregister(&self, device_id: Uuid, session: &Arc<DeviceSession>) {
        let is_same = self
            .devices
            .get(&device_id)
            .is_some_and(|entry| Arc::ptr_eq(entry.value(), session));

        if is_same {
            self.devices.remove(&device_id);
        }
    }

    pub fn get(&self, device_id: Uuid) -> Option<Arc<DeviceSession>> {
        self.devices.get(&device_id).map(|entry| entry.clone())
    }

    pub fn is_online(&self, device_id: Uuid) -> bool {
        self.devices.contains_key(&device_id)
    }
}

pub struct StreamRegistration {
    pub head_rx: oneshot::Receiver<ResponseHead>,
    pub body_rx: mpsc::Receiver<Result<Bytes, io::Error>>,
}

#[derive(Debug)]
pub struct ResponseHead {
    pub status: u16,
    pub headers: Headers,
}

pub struct DeviceSession {
    device_id: Uuid,
    max_streams: usize,
    frame_tx: mpsc::Sender<Frame>,
    shutdown: CancellationToken,
    streams: DashMap<Uuid, StreamResponder>,
}

impl DeviceSession {
    fn new(
        device_id: Uuid,
        max_streams: usize,
        frame_tx: mpsc::Sender<Frame>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            device_id,
            max_streams,
            frame_tx,
            shutdown,
            streams: DashMap::new(),
        }
    }

    pub fn shutdown(&self) {
        self.shutdown.cancel();
    }

    pub fn shutdown_token(&self) -> &CancellationToken {
        &self.shutdown
    }

    pub fn register_stream(
        &self,
        stream_id: Uuid,
        body_capacity: usize,
    ) -> Result<StreamRegistration> {
        if self.streams.len() >= self.max_streams {
            return Err(anyhow!("too many active streams"));
        }

        let (head_tx, head_rx) = oneshot::channel();
        let (body_tx, body_rx) = mpsc::channel(body_capacity);
        self.streams.insert(
            stream_id,
            StreamResponder {
                head_tx: Some(head_tx),
                body_tx,
            },
        );

        Ok(StreamRegistration { head_rx, body_rx })
    }

    pub async fn send_frame(&self, frame: Frame) -> Result<()> {
        self.frame_tx
            .send(frame)
            .await
            .map_err(|err| anyhow!("device {} channel closed: {err}", self.device_id))
    }

    pub async fn deliver_frame(&self, frame: Frame) -> Result<()> {
        match frame {
            Frame::ResponseHead {
                stream_id,
                status,
                headers,
            } => {
                let Some(mut entry) = self.streams.get_mut(&stream_id) else {
                    return Ok(());
                };
                let Some(head_tx) = entry.head_tx.take() else {
                    return Err(anyhow!("duplicate response head: {stream_id}"));
                };
                let _ = head_tx.send(ResponseHead { status, headers });
            }
            Frame::ResponseBodyChunk { stream_id, data } => {
                let Some(body_tx) = self
                    .streams
                    .get(&stream_id)
                    .map(|entry| entry.body_tx.clone())
                else {
                    return Ok(());
                };
                if body_tx.send(Ok(data)).await.is_err() {
                    self.streams.remove(&stream_id);
                }
            }
            Frame::ResponseBodyEnd { stream_id } => {
                self.streams.remove(&stream_id);
            }
            Frame::ErrorStream {
                stream_id,
                status,
                message,
            } => {
                let Some((_, responder)) = self.streams.remove(&stream_id) else {
                    return Ok(());
                };
                if let Some(head_tx) = responder.head_tx {
                    let _ = head_tx.send(ResponseHead {
                        status,
                        headers: HeaderMap::new(),
                    });
                }
                let _ = responder.body_tx.send(Err(io::Error::other(message))).await;
            }
            Frame::CancelStream { stream_id } => {
                self.streams.remove(&stream_id);
            }
            _ => {
                return Err(anyhow!("unexpected frame from device"));
            }
        }

        Ok(())
    }

    pub async fn cancel_stream(&self, stream_id: Uuid) {
        self.streams.remove(&stream_id);
        let _ = self.send_frame(Frame::CancelStream { stream_id }).await;
    }

    pub async fn close_all(&self, reason: &str) {
        let stream_ids: Vec<Uuid> = self.streams.iter().map(|entry| *entry.key()).collect();
        for stream_id in stream_ids {
            if let Some((_, responder)) = self.streams.remove(&stream_id) {
                if let Some(head_tx) = responder.head_tx {
                    let _ = head_tx.send(ResponseHead {
                        status: 503,
                        headers: HeaderMap::new(),
                    });
                }
                let _ = responder
                    .body_tx
                    .send(Err(io::Error::other(reason.to_owned())))
                    .await;
            }
        }
    }
}

struct StreamResponder {
    head_tx: Option<oneshot::Sender<ResponseHead>>,
    body_tx: mpsc::Sender<Result<Bytes, io::Error>>,
}
