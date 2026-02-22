use anyhow::Context;

use crate::api::config::ApiConfig;
use crate::api::state::ApiState;
use crate::sqlx::{PgConfig, SqlxClient};

pub mod config;
pub mod controllers;
pub mod endpoint;
pub mod models;
pub mod state;

pub async fn http_service(api_config: ApiConfig, pg_config: PgConfig) -> anyhow::Result<()> {
    let db_url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;
    let pool = ::sqlx::postgres::PgPoolOptions::new()
        .max_connections(pg_config.db_pool_size)
        .connect(&db_url)
        .await
        .context("failed to connect to PostgreSQL")?;

    tracing::info!(listen_addr = %api_config.listen_addr, "API server started");

    let state = ApiState::builder()
        .with_config(api_config)
        .with_http_client(reqwest::Client::new())
        .with_sqlx_client(SqlxClient::new(pool))
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
