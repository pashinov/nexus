use anyhow::Context;
use redis::AsyncCommands;

pub use self::config::RedisConfig;

mod config;

#[derive(Clone)]
pub struct RedisClient {
    client: redis::aio::ConnectionManager,
}

impl RedisClient {
    pub async fn new(url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(url).context("failed to create Redis client")?;

        let manager = redis::aio::ConnectionManager::new(client)
            .await
            .context("failed to connect to Redis")?;

        Ok(Self { client: manager })
    }

    /// Store a CSRF state token with a TTL.
    pub async fn store_oauth_state(&self, state: &str, ttl_secs: u64) -> anyhow::Result<()> {
        let mut conn = self.client.clone();
        let _: () = conn
            .set_ex(state, 1u8, ttl_secs)
            .await
            .context("failed to store OAuth state in Redis")?;
        Ok(())
    }

    /// Delete the CSRF state token and return whether it existed.
    pub async fn consume_oauth_state(&self, state: &str) -> anyhow::Result<bool> {
        let mut conn = self.client.clone();
        let deleted: i64 = conn
            .del(state)
            .await
            .context("failed to consume OAuth state from Redis")?;
        Ok(deleted > 0)
    }

    /// Add a JWT to the revocation blocklist with a TTL equal to its remaining lifetime.
    pub async fn revoke_jwt(&self, jti: &str, ttl_secs: u64) -> anyhow::Result<()> {
        let mut conn = self.client.clone();
        let _: () = conn
            .set_ex(format!("jwt:revoked:{jti}"), 1u8, ttl_secs)
            .await
            .context("failed to revoke JWT in Redis")?;
        Ok(())
    }

    /// Check whether a JWT has been revoked.
    pub async fn is_jwt_revoked(&self, jti: &str) -> anyhow::Result<bool> {
        let mut conn = self.client.clone();
        let exists: bool = conn
            .exists(format!("jwt:revoked:{jti}"))
            .await
            .context("failed to check JWT revocation in Redis")?;
        Ok(exists)
    }
}
