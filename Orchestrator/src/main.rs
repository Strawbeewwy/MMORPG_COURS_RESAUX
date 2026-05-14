mod config;
mod heartbeat;
mod process_manager;
mod redis_registry;
mod scaler;

use anyhow::{Context, Result};
use config::OrchestratorConfig;
use process_manager::ProcessManager;
use redis_registry::RedisRegistry;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let config = Arc::new(OrchestratorConfig::from_env()?);

    info!("orchestrator listening on {}", config.orch_addr);
    info!("redis url: {}", config.redis_url);
    info!("hot servers min: {}", config.hot_servers_min);

    let redis_client =
        redis::Client::open(config.redis_url.clone()).context("failed to create Redis client")?;

    let registry = Arc::new(RedisRegistry::new(redis_client));
    let process_manager = Arc::new(ProcessManager::new(config.first_ds_port));

    let heartbeat_task = tokio::spawn(heartbeat::heartbeat_listener(
        config.clone(),
        registry.clone(),
    ));

    let scaler_task = tokio::spawn(scaler::scaler_loop(
        config.clone(),
        registry.clone(),
        process_manager.clone(),
    ));

    tokio::select! {
        result = heartbeat_task => {
            result.context("heartbeat task join error")??;
        }
        result = scaler_task => {
            result.context("scaler task join error")??;
        }
    }

    Ok(())
}
