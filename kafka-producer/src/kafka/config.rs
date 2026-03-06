use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KafkaConfig {
    pub topic: String,
    pub brokers: String,
    pub message_timeout_ms: Option<u32>,
    pub message_max_size: Option<usize>,
    pub attempt_interval_ms: u64,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            topic: "device.info".to_owned(),
            brokers: "kafka:9092".to_owned(),
            message_timeout_ms: None,
            message_max_size: None,
            attempt_interval_ms: 1000,
        }
    }
}
