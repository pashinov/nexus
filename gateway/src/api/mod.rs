use anyhow::Context;

use crate::api::state::ApiState;
use crate::config::AppConfig;
use crate::redis::RedisClient;
use crate::sqlx::SqlxClient;

pub mod config;
pub mod controllers;
pub mod endpoint;
pub mod models;
pub mod state;

pub async fn http_service(config: AppConfig) -> anyhow::Result<()> {
    let db_url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;
    tracing::info!("connecting to PostgreSQL...");
    let pool = ::sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.postgres.db_pool_size)
        .connect(&db_url)
        .await
        .context("failed to connect to PostgreSQL")?;
    tracing::info!("PostgreSQL connected");

    tracing::info!("running database migrations");
    sqlx::migrate!("./migrations").run(&pool).await.context("failed to run database migrations")?;
    tracing::info!("database migrations complete");

    let redis_url = std::env::var("REDIS_URL").context("REDIS_URL not set")?;
    tracing::info!("connecting to Redis...");
    let redis_client = RedisClient::new(&redis_url)
        .await
        .context("failed to connect to Redis")?;
    tracing::info!("Redis connected");

    tracing::info!(listen_addr = %config.api.listen_addr, "API server starting...");

    let state = ApiState::builder()
        .with_config(config)
        .with_http_client(reqwest::Client::new())
        .with_sqlx_client(SqlxClient::new(pool))
        .with_redis_client(redis_client)
        .build()?;

    let endpoint = state.bind_endpoint().await?;

    tokio::task::spawn(async move {
        if let Err(e) = endpoint.serve().await {
            tracing::error!("API server failed: {e:?}");
        }
        tracing::info!("API server stopped");
    });

    Ok(())
}
