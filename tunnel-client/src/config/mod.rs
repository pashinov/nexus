use nexus_utils::logger::LoggerConfig;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub tunnel: TunnelConfig,
    pub logger: LoggerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TunnelConfig {
    /// WebSocket URL of the tunnel-server.
    /// Example: `ws://tunnel-server:8001`
    pub server_url: String,

    /// Unique device identifier — sent in the `device_id` query param.
    pub device_id: String,

    /// Base URL of the local HTTP service to proxy requests to.
    /// Example: `http://localhost:80`
    pub local_url: String,

    /// Seconds between reconnect attempts (base).
    pub reconnect_base_secs: u64,

    /// Maximum seconds to wait between reconnect attempts.
    pub reconnect_max_secs: u64,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            server_url: "ws://localhost:8001".to_owned(),
            device_id: "device-1".to_owned(),
            local_url: "http://localhost:80".to_owned(),
            reconnect_base_secs: 2,
            reconnect_max_secs: 60,
        }
    }
}
