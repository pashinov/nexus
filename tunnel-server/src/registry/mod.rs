use dashmap::DashMap;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::models::{TunnelRequest, TunnelResponse};

pub struct DeviceRegistry {
    /// device_id → sender channel to the device WebSocket handler
    devices: DashMap<Uuid, mpsc::Sender<TunnelRequest>>,
    /// request id → oneshot sender waiting for the device response
    pending: DashMap<Uuid, oneshot::Sender<TunnelResponse>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            devices: DashMap::new(),
            pending: DashMap::new(),
        }
    }

    pub fn register(&self, device_id: Uuid, tx: mpsc::Sender<TunnelRequest>) {
        self.devices.insert(device_id, tx);
    }

    pub fn unregister(&self, device_id: Uuid) {
        self.devices.remove(&device_id);
    }

    pub fn is_online(&self, device_id: Uuid) -> bool {
        self.devices.contains_key(&device_id)
    }

    pub async fn send_request(
        &self,
        device_id: Uuid,
        req: TunnelRequest,
        resp_tx: oneshot::Sender<TunnelResponse>,
    ) -> anyhow::Result<()> {
        let tx = self
            .devices
            .get(&device_id)
            .ok_or_else(|| anyhow::anyhow!("device not connected"))?
            .clone();

        let req_id = req.id;
        self.pending.insert(req_id, resp_tx);

        if let Err(e) = tx.send(req).await {
            self.pending.remove(&req_id);
            return Err(anyhow::anyhow!("device channel closed: {e}"));
        }

        Ok(())
    }

    pub fn complete_request(&self, resp: TunnelResponse) {
        if let Some((_, tx)) = self.pending.remove(&resp.id) {
            let _ = tx.send(resp);
        }
    }

    pub fn cancel_request(&self, req_id: Uuid) {
        self.pending.remove(&req_id);
    }
}
