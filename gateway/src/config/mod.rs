use nexus_utils::logger::LoggerConfig;
use serde::{Deserialize, Serialize};

use crate::api::config::ApiConfig;
use crate::kafka::KafkaConfig;
use crate::redis::RedisConfig;
use crate::sqlx::PgConfig;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub api: ApiConfig,

    pub postgres: PgConfig,

    pub redis: RedisConfig,

    pub kafka: KafkaConfig,

    pub logger: LoggerConfig,
}
