use crate::{
    config::OrchestratorConfig, process_manager::ProcessManager, redis_registry::RedisRegistry,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::time;
use tracing::{error, info};

pub async fn scaler_loop(
    config: Arc<OrchestratorConfig>,
    registry: Arc<RedisRegistry>,
    process_manager: Arc<ProcessManager>,
) -> Result<()> {
    ///the interval at which the scaler will run
    let mut interval = time::interval(time::Duration::from_secs(config.scaler_interval_seconds));

    ///main loop of the scaler
    loop {
        ///wait for the interval to elapse
        interval.tick().await;

        ///reap all the finished processes
        process_manager.reap_finished_processes().await;

        ///count the number of available servers
        let available = registry
            .count_available_servers()
            .await
            .unwrap_or_else(|err| {
                error!("failed to count available servers: {err:#}");
                0
            });

        info!(
            "available servers: {}, required hot servers: {}",
            available, config.hot_servers_min
        );

        if available < config.hot_servers_min {
            let to_spawn = config.hot_servers_min - available;

            for _ in 0..to_spawn {
                ///spawn servers as much as to_spawn needs
                match process_manager.spawn_server(&config).await {
                    Ok(port) => info!("spawned dedicated server on port {}", port),
                    Err(err) => error!("failed to spawn dedicated server: {err:#}"),
                }
            }
        }
    }
}
