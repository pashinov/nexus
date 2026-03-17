use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A proxied HTTP request sent from tunnel-server to device over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelRequest {
    pub id: Uuid,
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// A proxied HTTP response sent from device back to tunnel-server over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelResponse {
    pub id: Uuid,
    pub status: u16,
    pub headers: HashMap<String, String>,
    /// Base64-encoded response body.
    pub body: Option<String>,
}
