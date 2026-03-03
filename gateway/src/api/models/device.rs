use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Payload sent by EMQX Rule Engine webhook.
#[derive(Debug, Deserialize)]
pub struct DeviceInfoRequest {
    pub id: Uuid,
    pub client_version: String,
}

/// Payload for binding a device to the authenticated user.
#[derive(Debug, Deserialize)]
pub struct BindDeviceRequest {
    pub id: Uuid,
}

/// Device info returned to the user.
#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    pub id: Uuid,
    pub client_version: String,
    pub last_seen_at: DateTime<Utc>,
}
