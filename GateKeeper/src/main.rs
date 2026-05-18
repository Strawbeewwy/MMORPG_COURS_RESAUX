mod handlers;
mod redis_pool;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use shared::config::{GATEKEEPER_HTTP_ADDRESS, DEFAULT_REDIS_URL};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {

    tracing_subscriber::fmt::init();
    //get the redis url
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());

    //get the bind address we will use to listen for requests
    let bind_address = std::env::var("GATEKEEPER_HTTP_ADDRESS")
        .unwrap_or_else(|_| GATEKEEPER_HTTP_ADDRESS.to_string());

    let state = Arc::new(AppState {
        redis: redis_pool::create_pool(&redis_url)?,
    });

    let app = Router::new()
        //route for GET health
        .route("/health", get(handlers::health_handler))
        //route for POST login
        .route("/login", post(handlers::login_handler))
        .with_state(state);

    let addr: SocketAddr = bind_address.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("GateKeeper REST API listening on http://{addr}");

    axum::serve(listener, app).await?;

    Ok(())
}

pub struct AppState {
    pub redis: deadpool_redis::Pool,//deadpool are used for async redis connections
}
