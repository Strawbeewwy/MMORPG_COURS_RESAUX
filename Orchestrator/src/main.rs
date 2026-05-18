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
    /*
    Tracer used to keep track and debug the orchestrator
    */
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let config = Arc::new(OrchestratorConfig::from_env()?);

    info!("orchestrator listening on {}", config.orch_addr);
    info!("redis url: {}", config.redis_url);
    info!("hot servers min: {}", config.hot_servers_min);

    //sets up the redis client
    let redis_client =
        redis::Client::open(config.redis_url.clone()).context("failed to create Redis client")?;
    // sets up the redis registry
    let registry = Arc::new(RedisRegistry::new(redis_client));
    /*
    creates the process manager that can open or close
    dedicated servers
    */
    let process_manager = Arc::new(ProcessManager::new(config.first_ds_port));

    /*
    Task to listen to servers heartbeat messages
    using tokio to make it asynchronous
    */
    let heartbeat_task = tokio::spawn(heartbeat::heartbeat_listener(
        config.clone(),
        registry.clone(),
    ));
    /*
    Scaler is just a wrapper for the process manager
    it contains the loop that will decide when to spawn new dedicated servers
    */
    let scaler_task = tokio::spawn(scaler::scaler_loop(
        config.clone(),
        registry.clone(),
        process_manager.clone(),
    ));

    /*
    since 2 tasks are running at the same time and forever
    we dont want them to crash, so if any of them crashes
    we will know and act accordingly
    */
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
