use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PgConfig {
    /// Postgres connection pools.
    pub db_pool_size: u32,
}

impl Default for PgConfig {
    fn default() -> Self {
        Self { db_pool_size: 5 }
    }
}
