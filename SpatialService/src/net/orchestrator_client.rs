
use std::net::{SocketAddr, UdpSocket};
use std::str::FromStr;
use bevy::prelude::{Commands, Res};
use shared::{codec, OrchestratorCommand, ShardId};
use crate::resources::config::SpatialConfig;
use crate::resources::net_handles::OrchestratorClient;

pub fn connect_to_orchestrator(mut commands: Commands, config: Res<SpatialConfig>) {

    let addr_str = format!("{}:{}",config.orchestrator_host,config.orchestrator_port);
    let address = SocketAddr::from_str(&addr_str).unwrap();


    commands.insert_resource(OrchestratorClient::new(address));

    let spatial_info = "hello from spatial";

    let command = OrchestratorCommand::SpatialHello {
        spatial_info: spatial_info.to_string(),
    };

    let packet = codec::encode(&command).unwrap();

    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.send_to(&packet, address).unwrap();
}

pub fn send_spawn_server_request(
    command_addr: SocketAddr,
    count: u16,
    reason: impl Into<String>,
) -> anyhow::Result<()> {
    let command = OrchestratorCommand::SpawnServer {
        count,
        reason: reason.into(),
    };

    let packet = codec::encode(&command)?;

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.send_to(&packet, command_addr)?;

    Ok(())
}

pub fn maybe_request_server_for_shard(
    orchestrator: &mut OrchestratorClient,
    shard_id: ShardId,
    entity_count: usize,
) {
    if !orchestrator.should_request_server(shard_id, entity_count) {
        return;
    }

    if let Err(error) = send_spawn_server_request(
        orchestrator.command_addr,
        1,
        format!(
            "shard {} overloaded: {} entities",
            shard_id.0,
            entity_count
        ),
    ) {
        tracing::error!(
            "failed to request new server for overloaded shard {}: {error:#}",
            shard_id.0
        );
        return;
    }

    tracing::info!(
        "requested new server because shard {} has {} entities",
        shard_id.0,
        entity_count
    );
}
