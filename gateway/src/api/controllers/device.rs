use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::api::controllers::auth::AuthUser;
use crate::api::models::device::BindDeviceRequest;
use crate::api::state::ApiState;

/// GET /user/devices
/// List devices bound to the authenticated user.
pub async fn list(State(state): State<ApiState>, AuthUser(claims): AuthUser) -> impl IntoResponse {
    let user_id = match claims.sub.parse() {
        Ok(id) => id,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    match state.sqlx_client().get_user_devices(user_id).await {
        Ok(devices) => Json(devices).into_response(),
        Err(e) => {
            tracing::error!("failed to get user devices: {e:#}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
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
