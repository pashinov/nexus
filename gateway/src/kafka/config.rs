use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KafkaConfig {
    pub brokers: String,
    pub topic: String,
    pub group_id: String,
    pub client_id: String,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            brokers: "kafka:9092".to_owned(),
            topic: "device.info".to_owned(),
            group_id: "gateway".to_owned(),
            client_id: "gateway".to_owned(),
        }
    }
}
