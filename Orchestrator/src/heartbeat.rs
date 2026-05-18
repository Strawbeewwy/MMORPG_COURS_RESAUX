use crate::{config::OrchestratorConfig, redis_registry::RedisRegistry};
use anyhow::{Context, Result};
use shared::protocol::Heartbeat;
use shared::protocol::transport::codec;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{error, info, warn};
use shared::config::DEFAULT_HEARTBEAT_BUFFER_SIZE;

pub async fn heartbeat_listener(
    config: Arc<OrchestratorConfig>,
    registry: Arc<RedisRegistry>,
) -> Result<()> {
    /**we use udp for the heatbeat, since we just need
    acknoledgement that the server is alive all the info are
    just flourish
    **/
    let socket = UdpSocket::bind(config.orch_addr)
        .await
        .with_context(|| format!("failed to bind UDP socket on {}", config.orch_addr))?;

    ///this should be enough for the heartbeat
    let mut buffer = [0_u8; DEFAULT_HEARTBEAT_BUFFER_SIZE];

    ///main loop of the listener
    loop {
        /**
        when we receive a packet we put it in the buffer and then
        create a tuple with the length of the packet and the source ip address
        **/
        let (len, source) = socket
            .recv_from(&mut buffer)
            .await
            .context("failed to receive UDP heartbeat")?;


        ///decode the buffer into a heartbeat
        match codec::decode::<Heartbeat>(&buffer[..len]) {
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
                    error!(
                        "failed to update Redis for server {}: {err:#}",
                        heartbeat.id
                    );
                }
            }
            Err(err) => {
                warn!("invalid heartbeat from {}: {err:#}", source);
            }
        }
    }
}
