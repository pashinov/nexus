use nexus_utils::logger::LoggerConfig;
use serde::{Deserialize, Serialize};

use crate::mqtt::MqttConfig;
use crate::storage::StorageConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub mqtt: MqttConfig,

    pub storage: StorageConfig,

    pub logger: LoggerConfig,

    #[serde(with = "humantime_serde")]
    pub publish_info_interval: std::time::Duration,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mqtt: MqttConfig::default(),
            storage: StorageConfig::default(),
            logger: LoggerConfig::default(),
            publish_info_interval: std::time::Duration::from_secs(10),
        }
    }
}
