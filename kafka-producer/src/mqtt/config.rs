use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MqttConfig {
    /// MQTT broker host.
    pub host: String,
    /// MQTT broker port.
    pub port: u16,
    /// Keep-alive interval in seconds.
    pub keep_alive_secs: u16,
    /// Internal channel capacity for outgoing messages.
    pub channel_capacity: usize,
    /// Clean persistent session
    pub clean_session: bool,
    /// Path to the CA certificate PEM file.
    pub ca_cert: PathBuf,
    /// Path to the client certificate PEM file.
    pub client_cert: PathBuf,
    /// Path to the client private key PEM file.
    pub client_key: PathBuf,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_owned(),
            port: 8883,
            keep_alive_secs: 60,
            channel_capacity: 10,
            clean_session: true,
            ca_cert: PathBuf::from("/etc/mqtt/certs/ca.pem"),
            client_cert: PathBuf::from("/etc/mqtt/certs/client.pem"),
            client_key: PathBuf::from("/etc/mqtt/certs/client.key"),
        }
    }
}
