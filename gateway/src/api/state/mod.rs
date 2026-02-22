use std::collections::HashSet;
use std::sync::Arc;

use parking_lot::Mutex;

use anyhow::Result;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use reqwest::Client as HttpClient;
use tokio::net::TcpListener;

use crate::api::config::{ApiConfig, ApiSecrets};
use crate::api::endpoint::ApiEndpoint;
use crate::api::models::auth::Claims;
use crate::sqlx::SqlxClient;
use crate::utils::time::now_sec;

pub struct ApiStateBuilder<MandatoryFields = (HttpClient, SqlxClient)> {
    config: ApiConfig,
    mandatory_fields: MandatoryFields,
}

impl ApiStateBuilder {
    pub fn build(self) -> Result<ApiState> {
        let (http_client, sqlx_client) = self.mandatory_fields;
        let config = self.config;

        let secrets = ApiSecrets::from_env()?;

        Ok(ApiState {
            inner: Arc::new(Inner {
                config,
                secrets,
                http_client,
                sqlx_client,
                // TODO: replace with Redis for multi-server deployments
                oauth_states: Mutex::new(HashSet::new()),
            }),
        })
    }
}

impl<T2> ApiStateBuilder<((), T2)> {
    pub fn with_http_client(self, http_client: HttpClient) -> ApiStateBuilder<(HttpClient, T2)> {
        let (_, sqlx_client) = self.mandatory_fields;

        ApiStateBuilder {
            config: self.config,
            mandatory_fields: (http_client, sqlx_client),
        }
    }
}

impl<T1> ApiStateBuilder<(T1, ())> {
    pub fn with_sqlx_client(self, sqlx_client: SqlxClient) -> ApiStateBuilder<(T1, SqlxClient)> {
        let (http_client, _) = self.mandatory_fields;

        ApiStateBuilder {
            config: self.config,
            mandatory_fields: (http_client, sqlx_client),
        }
    }
}

impl<T1, T2> ApiStateBuilder<(T1, T2)> {
    pub fn with_config(self, config: ApiConfig) -> ApiStateBuilder<(T1, T2)> {
        ApiStateBuilder { config, ..self }
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct ApiState {
    inner: Arc<Inner>,
}

impl ApiState {
    pub fn builder() -> ApiStateBuilder<((), ())> {
        ApiStateBuilder {
            config: ApiConfig::default(),
            mandatory_fields: ((), ()),
        }
    }

    pub async fn bind_socket(&self) -> std::io::Result<TcpListener> {
        TcpListener::bind(self.config().listen_addr).await
    }

    pub async fn bind_endpoint(&self) -> Result<ApiEndpoint> {
        ApiEndpoint::builder().bind(self.clone()).await
    }

    pub fn config(&self) -> &ApiConfig {
        &self.inner.config
    }

    pub fn secrets(&self) -> &ApiSecrets {
        &self.inner.secrets
    }

    pub fn http_client(&self) -> &reqwest::Client {
        &self.inner.http_client
    }

    /// Store a CSRF state token generated on OAuth redirect.
    pub fn store_oauth_state(&self, state: &str) {
        self.inner.oauth_states.lock().insert(state.to_owned());
    }

    /// Consume a CSRF state token on callback.
    /// Returns false if state not found — possible CSRF attack.
    pub fn consume_oauth_state(&self, state: &str) -> bool {
        self.inner.oauth_states.lock().remove(state)
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
        let exp = now_sec() + self.inner.config.oauth.jwt.expires_in;

        let claims = Claims {
            sub: sub.to_owned(),
            email: email.to_owned(),
            name: name.to_owned(),
            exp,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.inner.secrets.jwt_secret.as_bytes()),
        )
        .map_err(Into::into)
    }
}

struct Inner {
    config: ApiConfig,
    secrets: ApiSecrets,
    sqlx_client: SqlxClient,
    http_client: reqwest::Client,
    // TODO: replace with Redis for multi-server deployments
    oauth_states: Mutex<HashSet<String>>,
}
