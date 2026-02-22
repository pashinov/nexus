use serde::{Deserialize, Serialize};

use crate::api::config::ApiConfig;
use crate::sqlx::PgConfig;
use crate::utils::logger::LoggerConfig;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub api: ApiConfig,

    pub postgres: PgConfig,

    pub logger: LoggerConfig,
}
