use nexus_utils::logger::LoggerConfig;
use serde::{Deserialize, Serialize};

use crate::mqtt::MqttConfig;
use crate::storage::StorageConfig;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub mqtt: MqttConfig,

    pub storage: StorageConfig,

    pub logger: LoggerConfig,
}
