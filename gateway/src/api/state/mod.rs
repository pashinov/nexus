use std::sync::Arc;

use anyhow::Result;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use reqwest::Client as HttpClient;
use tokio::net::TcpListener;

use uuid::Uuid;

use crate::api::config::{ApiConfig, ApiSecrets};
use crate::api::endpoint::ApiEndpoint;
use crate::api::models::auth::Claims;
use crate::config::AppConfig;
use crate::redis::{RedisClient, RedisConfig};
use crate::sqlx::SqlxClient;
use crate::utils::time::now_sec;

pub struct ApiStateBuilder<MandatoryFields = (HttpClient, SqlxClient, RedisClient)> {
    config: AppConfig,
    redis_config: RedisConfig,
    mandatory_fields: MandatoryFields,
}

impl ApiStateBuilder {
    pub fn build(self) -> Result<ApiState> {
        let (http_client, sqlx_client, redis_client) = self.mandatory_fields;
        let config = self.config;

        let secrets = ApiSecrets::from_env()?;

        Ok(ApiState {
            inner: Arc::new(Inner {
                config,
                secrets,
                sqlx_client,
                redis_client,
                http_client,
            }),
        })
    }
}

impl<T2, T3> ApiStateBuilder<((), T2, T3)> {
    pub fn with_http_client(
        self,
        http_client: HttpClient,
    ) -> ApiStateBuilder<(HttpClient, T2, T3)> {
        let (_, sqlx_client, redis_client) = self.mandatory_fields;
        ApiStateBuilder {
            config: self.config,
            redis_config: self.redis_config,
            mandatory_fields: (http_client, sqlx_client, redis_client),
        }
    }
}

impl<T1, T3> ApiStateBuilder<(T1, (), T3)> {
    pub fn with_sqlx_client(
        self,
        sqlx_client: SqlxClient,
    ) -> ApiStateBuilder<(T1, SqlxClient, T3)> {
        let (http_client, _, redis_client) = self.mandatory_fields;
        ApiStateBuilder {
            config: self.config,
            redis_config: self.redis_config,
            mandatory_fields: (http_client, sqlx_client, redis_client),
        }
    }
}

impl<T1, T2> ApiStateBuilder<(T1, T2, ())> {
    pub fn with_redis_client(
        self,
        redis_client: RedisClient,
    ) -> ApiStateBuilder<(T1, T2, RedisClient)> {
        let (http_client, sqlx_client, _) = self.mandatory_fields;
        ApiStateBuilder {
            config: self.config,
            redis_config: self.redis_config,
            mandatory_fields: (http_client, sqlx_client, redis_client),
        }
    }
}

impl<T1, T2, T3> ApiStateBuilder<(T1, T2, T3)> {
    pub fn with_config(self, config: AppConfig) -> ApiStateBuilder<(T1, T2, T3)> {
        ApiStateBuilder { config, ..self }
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct ApiState {
    inner: Arc<Inner>,
}

impl ApiState {
    pub fn builder() -> ApiStateBuilder<((), (), ())> {
        ApiStateBuilder {
            config: AppConfig::default(),
            redis_config: RedisConfig::default(),
            mandatory_fields: ((), (), ()),
        }
    }

    pub async fn bind_socket(&self) -> std::io::Result<TcpListener> {
        TcpListener::bind(self.api_config().listen_addr).await
    }

    pub async fn bind_endpoint(&self) -> Result<ApiEndpoint> {
        ApiEndpoint::builder().bind(self.clone()).await
    }

    pub fn api_config(&self) -> &ApiConfig {
        &self.inner.config.api
    }

    pub fn secrets(&self) -> &ApiSecrets {
        &self.inner.secrets
    }

    pub fn http_client(&self) -> &reqwest::Client {
        &self.inner.http_client
    }

    pub fn sqlx_client(&self) -> &SqlxClient {
        &self.inner.sqlx_client
    }

    /// Store a CSRF state token generated on OAuth redirect.
    pub async fn store_oauth_state(&self, state: &str) -> anyhow::Result<()> {
        self.inner
            .redis_client
            .store_oauth_state(state, self.inner.config.redis.oauth_state_ttl)
            .await
    }

    /// Consume a CSRF state token on callback.
    /// Returns false if state not found — possible CSRF attack.
    pub async fn consume_oauth_state(&self, state: &str) -> anyhow::Result<bool> {
        self.inner.redis_client.consume_oauth_state(state).await
    }

    pub fn decode_jwt(&self, token: &str) -> Result<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.inner.secrets.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(Into::into)
    }

    pub fn issue_jwt(&self, sub: &str, email: &str, name: &str) -> Result<String> {
        let exp = now_sec() + self.inner.config.api.oauth.jwt.expires_in;

        let claims = Claims {
            sub: sub.to_owned(),
            email: email.to_owned(),
            name: name.to_owned(),
            exp,
            jti: Uuid::new_v4().to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.inner.secrets.jwt_secret.as_bytes()),
        )
        .map_err(Into::into)
    }

    /// Add a JWT to the revocation blocklist for its remaining lifetime.
    pub async fn revoke_jwt(&self, claims: &Claims) -> anyhow::Result<()> {
        let remaining = claims.exp.saturating_sub(now_sec()) as u64;
        if remaining == 0 {
            return Ok(());
        }
        self.inner
            .redis_client
            .revoke_jwt(&claims.jti, remaining)
            .await
    }

    /// Returns true if the JWT has been explicitly revoked.
    pub async fn is_jwt_revoked(&self, jti: &str) -> anyhow::Result<bool> {
        self.inner.redis_client.is_jwt_revoked(jti).await
    }
}

struct Inner {
    config: AppConfig,
    secrets: ApiSecrets,
    sqlx_client: SqlxClient,
    redis_client: RedisClient,
    http_client: reqwest::Client,
}
