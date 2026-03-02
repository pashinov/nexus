use axum::Json;
use axum::response::IntoResponse;

use crate::api::controllers::auth::AuthUser;

pub async fn info(AuthUser(claims): AuthUser) -> impl IntoResponse {
    Json(claims)
}
