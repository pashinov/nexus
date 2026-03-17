use anyhow::{Context, Result};
use tokio_util::sync::CancellationToken;

use crate::config::AppConfig;
use crate::redis::RedisClient;
use crate::state::TunnelState;

pub mod controllers;
pub mod endpoint;

pub async fn http_service(config: AppConfig, token: CancellationToken) -> Result<()> {
    tracing::info!("connecting to Redis...");
    let redis_client = RedisClient::new(&config.redis.url)
        .await
        .context("failed to connect to Redis")?;
    tracing::info!("Redis connected");

    tracing::info!(listen_addr = %config.api.listen_addr, "tunnel-server starting...");

    let state = TunnelState::builder()
        .with_config(config)
        .with_redis_client(redis_client)
        .build()?;

    let endpoint = state.bind_endpoint().await?;

    endpoint.serve(token).await?;

    tracing::info!("tunnel-server stopped");

    Ok(())
}
