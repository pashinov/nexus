use std::net::{Ipv4Addr, SocketAddr};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    /// TCP socket address to listen for incoming connections.
    ///
    /// Default: `0.0.0.0:8000`
    pub listen_addr: SocketAddr,

    pub oauth: OAuthConfig,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            listen_addr: (Ipv4Addr::UNSPECIFIED, 8000).into(),
            oauth: OAuthConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OAuthConfig {
    pub jwt: JwtConfig,

    /// Base URL of this service, used to construct redirect URIs.
    ///
    /// Example: `https://api.apashinov.com`
    pub base_url: String,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            jwt: JwtConfig::default(),
            base_url: "http://localhost:8000".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct JwtConfig {
    pub expires_in: u32,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self { expires_in: 86400 }
    }
}

/// Sensitive credentials — loaded exclusively from environment variables.
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct ApiSecrets {
    pub jwt_secret: String,
    pub client_id: String,
    pub client_secret: String,
}

impl ApiSecrets {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            jwt_secret: std::env::var("JWT_SECRET").context("JWT_SECRET not set")?,
            client_id: std::env::var("CLIENT_ID").context("CLIENT_ID not set")?,
            client_secret: std::env::var("CLIENT_SECRET").context("CLIENT_SECRET not set")?,
        })
    }
}
