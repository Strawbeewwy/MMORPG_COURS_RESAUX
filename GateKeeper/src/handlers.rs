use crate::redis_pool::find_available_server;
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use shared::protocol::{
    ErrorResponse, HealthResponse, LoginHttpRequest, LoginHttpResponse,
};
use std::sync::Arc;
use uuid::Uuid;

pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginHttpRequest>,
) -> Result<Json<LoginHttpResponse>, (StatusCode, Json<ErrorResponse>)> {
    if request.username.trim().is_empty() || request.password != "1234" {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid credentials".to_string(),
            }),
        ));
    }

    let server = find_available_server(&state.redis)
        .await
        .map_err(|error| {
            tracing::error!("failed to find available server: {error:?}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "No server available".to_string(),
                }),
            )
        })?;

    Ok(Json(LoginHttpResponse {
        player_id: Uuid::new_v4().to_string(),
        server,
    }))
}