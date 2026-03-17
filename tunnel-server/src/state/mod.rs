use std::sync::Arc;

use anyhow::{Context, Result};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use crate::api::endpoint::TunnelEndpoint;
use crate::config::{ApiConfig, AppConfig, AppSecrets};
use crate::redis::RedisClient;
use crate::registry::DeviceRegistry;

/// JWT claims — must match the gateway's structure.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub name: String,
    pub exp: u64,
    pub jti: String,
}

// ── Builder ───────────────────────────────────────────────────────────────

pub struct TunnelStateBuilder<R = RedisClient> {
    config: AppConfig,
    mandatory_fields: R,
}

impl TunnelStateBuilder {
    pub fn build(self) -> Result<TunnelState> {
        let secrets = AppSecrets::from_env()?;
        let decoding_key = DecodingKey::from_rsa_pem(secrets.jwt_public_key.as_bytes())
            .context("invalid JWT public key")?;

        Ok(TunnelState {
            inner: Arc::new(Inner {
                config: self.config,
                decoding_key,
                registry: Arc::new(DeviceRegistry::new()),
                redis_client: self.mandatory_fields,
            }),
        })
    }
}

impl<R> TunnelStateBuilder<R> {
    pub fn with_config(self, config: AppConfig) -> TunnelStateBuilder<R> {
        TunnelStateBuilder { config, ..self }
    }
}

impl TunnelStateBuilder<()> {
    pub fn with_redis_client(self, redis_client: RedisClient) -> TunnelStateBuilder<RedisClient> {
        TunnelStateBuilder {
            config: self.config,
            mandatory_fields: redis_client,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
#[repr(transparent)]
pub struct TunnelState {
    inner: Arc<Inner>,
}

impl TunnelState {
    pub fn builder() -> TunnelStateBuilder<()> {
        TunnelStateBuilder {
            config: AppConfig::default(),
            mandatory_fields: (),
        }
    }

    pub fn api_config(&self) -> &ApiConfig {
        &self.inner.config.api
    }

    pub fn registry(&self) -> &Arc<DeviceRegistry> {
        &self.inner.registry
    }

    pub fn redis(&self) -> &RedisClient {
        &self.inner.redis_client
    }

    pub fn decode_jwt(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        decode::<Claims>(token, &self.inner.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(Into::into)
    }

    pub async fn bind_socket(&self) -> std::io::Result<TcpListener> {
        TcpListener::bind(self.api_config().listen_addr).await
    }

    pub async fn bind_endpoint(&self) -> Result<TunnelEndpoint> {
        TunnelEndpoint::builder().bind(self.clone()).await
    }
}

struct Inner {
    config: AppConfig,
    decoding_key: DecodingKey,
    registry: Arc<DeviceRegistry>,
    redis_client: RedisClient,
}
