
use std::net::{SocketAddr, UdpSocket};
use std::str::FromStr;
use bevy::prelude::{Commands, Res, Vec2};
use shared::{codec, ClientId, EntityId, OrchestratorCommand, ShardId};
use crate::resources::config::SpatialConfig;
use crate::resources::entity_map::EntityMap;
use crate::resources::handoff_queue::PendingHandoffs;
use crate::resources::net_handles::OrchestratorClient;
use crate::resources::quad_tree::QuadTree;


#[derive(Debug, Clone, Copy)]
pub struct SplitMovedEntity {
    pub entity_id: EntityId,
    pub client_id: ClientId,
    pub position: Vec2,
    pub old_shard: ShardId,
    pub new_shard: ShardId,
}

#[derive(Debug, Clone)]
pub struct SplitShardResult {
    pub old_shard: ShardId,
    pub new_shards: [ShardId; 4],
    pub moved_entities: Vec<SplitMovedEntity>,
}



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

    if let Err(error) = send_spawn_shard_servers_request(
        address,
        [ShardId(0)],
        "initial root shard",
    ) {
        tracing::error!("failed to request initial root shard server: {error:#}");
    }
}

pub fn maybe_request_server_for_shard(
    orchestrator: &mut OrchestratorClient,
    shard_id: ShardId,
    entity_count: usize,
) {
    if !orchestrator.should_request_server(shard_id, entity_count) {
        return;
    }

    if let Err(error) = send_spawn_shard_servers_request(
        orchestrator.command_addr,
        [shard_id],
        format!(
            "shard {} overloaded: {} entities",
            shard_id.0,
            entity_count
        ),
    ) {
        tracing::error!(
            "failed to request server for overloaded shard {}: {error:#}",
            shard_id.0
        );
        return;
    }

    tracing::info!(
        "requested new server for shard {} because it has {} entities",
        shard_id.0,
        entity_count
    );
}

pub fn send_spawn_shard_servers_request(
    command_addr: SocketAddr,
    shard_ids: impl IntoIterator<Item = ShardId>,
    reason: impl Into<String>,
) -> anyhow::Result<()> {
    let shard_ids = shard_ids
        .into_iter()
        .map(|shard_id| shard_id.0)
        .collect();

    let command = OrchestratorCommand::SpawnShardServers {
        shard_ids,
        reason: reason.into(),
    };

    let packet = codec::encode(&command)?;

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.send_to(&packet, command_addr)?;

    Ok(())
}

pub fn request_servers_for_new_shards(
    orchestrator: &mut OrchestratorClient,
    shard_ids: [ShardId; 4],
    reason: impl Into<String>,
) {
    if let Err(error) = send_spawn_shard_servers_request(
        orchestrator.command_addr,
        shard_ids,
        reason,
    ) {
        tracing::error!("failed to request servers for new shards: {error:#}");
        return;
    }

    tracing::info!(
        "requested servers for new shards [{}, {}, {}, {}]",
        shard_ids[0].0,
        shard_ids[1].0,
        shard_ids[2].0,
        shard_ids[3].0,
    );
}

pub fn split_overloaded_shard_if_needed(
    quad_tree: &mut QuadTree,
    entity_map: &EntityMap,
    orchestrator: &mut OrchestratorClient,
    shard_id: ShardId,
    entity_count: usize,
) -> Option<SplitShardResult> {
    if entity_count <= orchestrator.max_entities_per_shard {
        return None;
    }

    let Some(new_shards) = quad_tree.split_shard(shard_id) else {
        maybe_request_server_for_shard(orchestrator, shard_id, entity_count);
        return None;
    };

    request_servers_for_new_shards(
        orchestrator,
        new_shards,
        format!(
            "split overloaded shard {} with {} entities",
            shard_id.0,
            entity_count
        ),
    );

    let mut moved_entities = Vec::new();

    for record in entity_map.entities.values() {
        if record.current_shard != shard_id {
            continue;
        }

        if !entity_map.is_stable(record.entity_id) {
            continue;
        }

        let Some(new_shard) = quad_tree.shard_for(record.position.x, record.position.y) else {
            continue;
        };

        if new_shard == shard_id {
            continue;
        }

        moved_entities.push(SplitMovedEntity {
            entity_id: record.entity_id,
            client_id: record.client_id,
            position: record.position,
            old_shard: shard_id,
            new_shard,
        });
    }

    tracing::info!(
        "split overloaded shard {} into shards [{}, {}, {}, {}], queued {} handoff(s)",
        shard_id.0,
        new_shards[0].0,
        new_shards[1].0,
        new_shards[2].0,
        new_shards[3].0,
        moved_entities.len(),
    );

    Some(SplitShardResult {
        old_shard: shard_id,
        new_shards,
        moved_entities,
    })
}

pub fn send_stop_shard_servers_request(
    command_addr: SocketAddr,
    shard_ids: impl IntoIterator<Item = ShardId>,
    reason: impl Into<String>,
) -> anyhow::Result<()> {
    let shard_ids = shard_ids
        .into_iter()
        .map(|shard_id| shard_id.0)
        .collect();

    let command = OrchestratorCommand::StopShardServers {
        shard_ids,
        reason: reason.into(),
    };

    let packet = codec::encode(&command)?;

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.send_to(&packet, command_addr)?;

    Ok(())
}

pub fn maybe_request_stop_shard_if_drained(
    orchestrator: &mut OrchestratorClient,
    entity_map: &EntityMap,
    pending_handoffs: &PendingHandoffs,
    shard_id: ShardId,
) {
    if !orchestrator.is_split_parent_candidate(shard_id) {
        return;
    }

    if entity_map.shard_count(shard_id) > 0 {
        return;
    }

    if pending_handoffs.pending_count_for(shard_id) > 0 {
        return;
    }

    if !orchestrator.should_request_stop_shard(shard_id) {
        return;
    }

    if let Err(error) = send_stop_shard_servers_request(
        orchestrator.command_addr,
        [shard_id],
        format!("retire split parent shard {}", shard_id.0),
    ) {
        tracing::error!(
            "failed to request stop for drained shard {}: {error:#}",
            shard_id.0
        );
        return;
    }

    orchestrator.clear_split_parent_candidate(shard_id);

    tracing::info!(
        "requested stop for drained split parent shard {}",
        shard_id.0
    );
}
