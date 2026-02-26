use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RedisConfig {
    /// TTL in seconds for OAuth CSRF state tokens.
    pub oauth_state_ttl: u64,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            oauth_state_ttl: 300,
        }
    }
}
