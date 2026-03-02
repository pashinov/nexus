use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::api::controllers::auth::AuthUser;
use crate::api::models::device::{BindDeviceRequest, DeviceInfoRequest};
use crate::api::state::ApiState;

/// POST /internal/device/info
/// Called by EMQX Rule Engine when a device publishes telemetry.
pub async fn info(
    State(state): State<ApiState>,
    Json(req): Json<DeviceInfoRequest>,
) -> impl IntoResponse {
    match state
        .sqlx_client()
        .upsert_device(req.id, &req.client_version)
        .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            tracing::error!("failed to upsert device: {e:#}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

/// POST /user/devices
/// Bind a device to the authenticated user. Device must exist (be online).
pub async fn bind(
    State(state): State<ApiState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<BindDeviceRequest>,
) -> impl IntoResponse {
    let user_id = match claims.sub.parse() {
        Ok(id) => id,
        Err(_) => return StatusCode::UNAUTHORIZED,
    };

    match state.sqlx_client().bind_device(user_id, req.id).await {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            tracing::error!("failed to bind device: {e:#}");
            StatusCode::UNPROCESSABLE_ENTITY
        }
    }
}
