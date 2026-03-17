use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RedisConfig {
    pub url: String,
    /// TTL in seconds for OAuth CSRF state tokens.
    pub oauth_state_ttl: u64,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_owned(),
            oauth_state_ttl: 300,
        }
    }
}
