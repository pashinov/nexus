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
    /// Clean persistent session.
    pub clean_session: bool,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            host: "emqx".to_owned(),
            port: 1883,
            keep_alive_secs: 60,
            channel_capacity: 10,
            clean_session: false,
        }
    }
}
