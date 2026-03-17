use anyhow::Context;
use redis::AsyncCommands;

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

    /// Store `session:{token} → device_id` with a TTL.
    pub async fn store_session(
        &self,
        token: &str,
        device_id: &str,
        ttl_secs: u64,
    ) -> anyhow::Result<()> {
        let mut conn = self.client.clone();
        let _: () = conn
            .set_ex(format!("session:{token}"), device_id, ttl_secs)
            .await
            .context("failed to store session in Redis")?;
        Ok(())
    }

    /// Look up the device_id associated with a session token.
    pub async fn get_session(&self, token: &str) -> anyhow::Result<Option<String>> {
        let mut conn = self.client.clone();
        let device_id: Option<String> = conn
            .get(format!("session:{token}"))
            .await
            .context("failed to get session from Redis")?;
        Ok(device_id)
    }
}
