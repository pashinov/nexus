use serde::Deserialize;
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
