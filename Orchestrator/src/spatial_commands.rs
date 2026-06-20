use crate::{config::OrchestratorConfig, process_manager::ProcessManager};
use anyhow::{Context, Result};
use shared::config::DEFAULT_HEARTBEAT_BUFFER_SIZE;
use shared::protocol::http::codec;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{error, info, warn};
use shared::OrchestratorCommand;

pub async fn spatial_command_listener(
    config: Arc<OrchestratorConfig>,
    process_manager: Arc<ProcessManager>,
) -> Result<()> {
    let mut command_addr = config.orch_addr;
    command_addr.set_port(config.orch_addr.port() + 1);

    let socket = UdpSocket::bind(command_addr)
        .await
        .with_context(|| format!("failed to bind spatial command UDP socket on {command_addr}"))?;

    info!("orchestrator spatial command listener on {}", command_addr);

    let mut buffer = [0_u8; DEFAULT_HEARTBEAT_BUFFER_SIZE];

    loop {
        let (len, source) = socket
            .recv_from(&mut buffer)
            .await
            .context("failed to receive spatial command")?;

        let command = match codec::decode::<OrchestratorCommand>(&buffer[..len]) {
            Ok(command) => command,
            Err(err) => {
                warn!("invalid spatial command from {}: {err:#}", source);
                continue;
            }
        };

        match command {
            OrchestratorCommand::SpawnServer { count, reason } => {
                info!(
                        "spatial requested {} new server(s), reason={}",
                        count,
                        reason
                    );

                for _ in 0..count {
                    match process_manager.spawn_server(&config).await {
                        Ok(port) => info!("spawned dedicated server from spatial request on port {}", port),
                        Err(err) => error!("failed to spawn dedicated server from spatial request: {err:#}"),
                    }
                }
            }
            OrchestratorCommand::SpawnShardServers { shard_ids, reason } => {
                info!(
                        "spatial requested shard server(s) for {:?}, reason={}",
                        shard_ids,
                        reason
                    );

                for shard_id in shard_ids {
                    match process_manager.spawn_server_for_shard(&config, Some(shard_id)).await {
                        Ok(port) => info!(
                                "spawned dedicated server for shard {} on port {}",
                                shard_id,
                                port
                            ),
                        Err(err) => error!(
                                "failed to spawn dedicated server for shard {}: {err:#}",
                                shard_id
                            ),
                    }
                }
            }
            OrchestratorCommand::StopShardServers { shard_ids, reason } => {
                info!(
                    "spatial requested shard stop for {:?}, reason={}",
                    shard_ids,
                    reason
                );

                match process_manager.stop_shard_servers(&shard_ids).await {
                    Ok(stopped) => info!("stopped {} shard server(s) from request", stopped),
                    Err(err) => error!("failed to stop shard server(s): {err:#}"),
                }
            }
            OrchestratorCommand::SpatialHello { spatial_info } => {
                info!(spatial_info);
            }
        }
    }
}