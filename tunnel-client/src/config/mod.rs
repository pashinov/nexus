use nexus_utils::logger::LoggerConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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

    /// Seconds between reconnect attempts
    #[serde(with = "humantime_serde")]
    pub reconnect_timeout: Duration,

    /// Maximum number of concurrent proxied streams on one device connection.
    pub max_concurrent_streams: usize,

    /// Outbound frame queue capacity for responses and control frames.
    pub frame_channel_capacity: usize,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            server_url: "ws://localhost:8001".to_owned(),
            device_id: "device-1".to_owned(),
            local_url: "http://localhost:80".to_owned(),
            reconnect_timeout: Duration::from_secs(5),
            max_concurrent_streams: 64,
            frame_channel_capacity: 64,
        }
    }
}
