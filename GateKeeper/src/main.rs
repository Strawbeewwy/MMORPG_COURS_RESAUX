mod handlers;
mod redis_pool;

use anyhow::Result;
use axum::{routing::{get, post}, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use shared::config::GATEKEEPER_HTTP_ADDRESS;


#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let bind_address = std::env::var("GATEKEEPER_HTTP_ADDRESS")
        .unwrap_or_else(|_| GATEKEEPER_HTTP_ADDRESS.to_string());

    let state = Arc::new(AppState {
        redis: redis_pool::create_pool(&redis_url)?,
    });

    let app = Router::new()
        .route("/health", get(handlers::health_handler))
        .route("/login", post(handlers::login_handler))
        .with_state(state);

    let addr: SocketAddr = bind_address.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("GateKeeper REST API listening on http://{addr}");

    axum::serve(listener, app).await?;

    Ok(())
}

pub struct AppState {
    pub redis: deadpool_redis::Pool,
}