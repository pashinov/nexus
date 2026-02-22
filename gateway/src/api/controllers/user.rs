use axum::Json;
use axum::response::IntoResponse;

use crate::api::controllers::auth::AuthUser;
use crate::api::models::user::{Device, DevicesRequest};

pub async fn info(AuthUser(claims): AuthUser) -> impl IntoResponse {
    Json(claims)
}

pub async fn devices(
    AuthUser(_claims): AuthUser,
    Json(_req): Json<DevicesRequest>,
) -> impl IntoResponse {
    Json(Vec::<Device>::new())
}
