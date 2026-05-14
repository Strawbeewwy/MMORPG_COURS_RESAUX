use crate::{config::OrchestratorConfig, redis_registry::RedisRegistry};
use anyhow::{Context, Result};
use shared::protocol::Heartbeat;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{error, info, warn};

pub async fn heartbeat_listener(
    config: Arc<OrchestratorConfig>,
    registry: Arc<RedisRegistry>,
) -> Result<()> {
    let socket = UdpSocket::bind(config.orch_addr)
        .await
        .with_context(|| format!("failed to bind UDP socket on {}", config.orch_addr))?;

    let mut buffer = [0_u8; 4096];

    loop {
        let (len, source) = socket
            .recv_from(&mut buffer)
            .await
            .context("failed to receive UDP heartbeat")?;

        let payload = std::str::from_utf8(&buffer[..len])
            .context("heartbeat was not valid UTF-8")?;

        match parse_heartbeat(payload) {
            Ok(heartbeat) => {
                info!(
                    "heartbeat from {}: id={} {}:{} players={}/{} status={}",
                    source,
                    heartbeat.id,
                    heartbeat.ip,
                    heartbeat.port,
                    heartbeat.player_count,
                    heartbeat.max_players,
                    heartbeat.status(),
                );

                if let Err(err) = registry
                    .update_server(&heartbeat, config.server_ttl_seconds)
                    .await
                {
                    error!("failed to update Redis for server {}: {err:#}", heartbeat.id);
                }
            }
            Err(err) => {
                warn!("invalid heartbeat from {}: {err:#}; payload={payload:?}", source);
            }
        }
    }
}

fn parse_heartbeat(payload: &str) -> Result<Heartbeat> {
    let trimmed = payload.trim();

    if let Some(json) = trimmed.strip_prefix("HEARTBEAT ") {
        serde_json::from_str(json).context("failed to parse HEARTBEAT JSON payload")
    } else {
        serde_json::from_str(trimmed).context("failed to parse heartbeat JSON payload")
    }
}