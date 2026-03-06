use nexus_utils::logger::LoggerConfig;
use serde::{Deserialize, Serialize};

use crate::kafka::KafkaConfig;
use crate::mqtt::MqttConfig;
use crate::storage::StorageConfig;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub mqtt: MqttConfig,
    pub kafka: KafkaConfig,
    pub storage: StorageConfig,
    pub logger: LoggerConfig,

    pub mqtt_topic: String,
}
