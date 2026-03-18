use std::net::{Ipv4Addr, SocketAddr};

use anyhow::Context;
use base64::Engine as _;
use nexus_utils::logger::LoggerConfig;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub api: ApiConfig,
    pub redis: RedisConfig,
    pub logger: LoggerConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    /// TCP socket address to listen for incoming connections.
    pub listen_addr: SocketAddr,
    /// Scheme used to construct tunnel session URLs: "http" or "https".
    pub tunnel_scheme: String,
    /// Domain used to construct tunnel session URLs.
    /// Local:  "localhost:8001"  → http://{token}.localhost:8001/
    /// Prod:   "tunnel.example.com" → https://{token}.tunnel.example.com/
    pub tunnel_domain: String,
    /// Session token TTL in seconds.
    pub session_ttl: u64,
    /// Maximum number of concurrent active streams per connected device.
    pub max_concurrent_streams_per_device: usize,
    /// Maximum frame chunk size for request and response bodies.
    pub max_chunk_size_bytes: usize,
    /// Channel capacity used for per-stream response buffering.
    pub stream_channel_capacity: usize,
    /// Maximum seconds to wait for the first response head frame.
    pub response_head_timeout_secs: u64,
    /// CORS allowed origins. Empty = permissive (all origins allowed).
    pub cors_origins: Vec<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            listen_addr: (Ipv4Addr::UNSPECIFIED, 8001).into(),
            tunnel_scheme: "http".to_owned(),
            tunnel_domain: "localhost:8001".to_owned(),
            session_ttl: 3600,
            max_concurrent_streams_per_device: 64,
            max_chunk_size_bytes: 64 * 1024,
            stream_channel_capacity: 16,
            response_head_timeout_secs: 30,
            cors_origins: vec![],
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RedisConfig {
    pub url: String,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_owned(),
        }
    }
}

/// Sensitive credentials — loaded exclusively from environment variables.
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct AppSecrets {
    pub jwt_public_key: String,
}

impl AppSecrets {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            jwt_public_key: decode_b64_env("JWT_PUBLIC_KEY")?,
        })
    }
}

fn decode_b64_env(var: &str) -> anyhow::Result<String> {
    let encoded = std::env::var(var).with_context(|| format!("{var} not set"))?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .with_context(|| format!("{var} is not valid base64"))?;
    String::from_utf8(bytes).with_context(|| format!("{var} is not valid UTF-8"))
}
