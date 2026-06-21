use crate::redis_pool::find_available_server;
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use shared::protocol::{ErrorResponse, HealthResponse, LoginHttpRequest, LoginHttpResponse};
use shared::config::DEFAULT_DEBUG_PASSWORD;
use anyhow::Result;
use std::sync::Arc;
use shared::{DEFAULT_BROKER_HOST, DEFAULT_BROKER_PORT};
use crate::broker_client::register_client_with_broker;

pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginHttpRequest>,
) -> Result<Json<LoginHttpResponse>, (StatusCode, Json<ErrorResponse>)> {
    if request.username.trim().is_empty() || request.password != DEFAULT_DEBUG_PASSWORD.to_string() {
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

    let username = request.username.clone();

    let client_id = tokio::task::spawn_blocking(move || {
        let broker_host =
            std::env::var("BROKER_HOST").unwrap_or_else(|_| DEFAULT_BROKER_HOST.to_string());

        let broker_port = std::env::var("BROKER_PORT")
            .ok()
            .and_then(|port| port.parse::<u16>().ok())
            .unwrap_or(DEFAULT_BROKER_PORT);

        register_client_with_broker(&broker_host, broker_port, &username)
    })
        .await
        .map_err(|error| {
            tracing::error!("failed to join Broker registration task: {error}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?
        .map_err(|error| {
            tracing::error!("failed to register client with Broker: {error:?}");

            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "Broker unavailable".to_string(),
                }),
            )
        })?;

    Ok(Json(LoginHttpResponse {
        client_id: client_id.0,
    }))
}
