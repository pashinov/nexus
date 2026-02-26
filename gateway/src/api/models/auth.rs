use serde::{Deserialize, Serialize};

/// Query parameters received on OAuth callback.
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: String,
}

/// Response returned to the client after successful authentication.
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
}

/// JWT claims.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — user ID from the OAuth provider.
    pub sub: String,
    pub email: String,
    pub name: String,
    /// Expiration timestamp (Unix seconds).
    pub exp: u32,
    /// Unique token ID used for revocation.
    pub jti: String,
}

/// User info returned by Google's userinfo endpoint.
#[derive(Debug, Deserialize)]
pub struct UserInfo {
    pub sub: String,
    pub email: String,
    pub name: String,
}

/// Token response from OAuth provider's token endpoint.
#[derive(Debug, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
}
