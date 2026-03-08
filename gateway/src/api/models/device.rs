use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Payload for binding a device to the authenticated user.
#[derive(Debug, Deserialize)]
pub struct BindDeviceRequest {
    pub id: Uuid,
}

/// Device info returned to the user.
#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    pub id: Uuid,
    pub uptime: i64,
    pub info: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
