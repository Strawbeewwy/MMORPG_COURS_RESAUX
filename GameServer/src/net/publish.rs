use bevy::platform::collections::HashMap;
use bevy::prelude::{Query, Res, ResMut, Resource};
use shared::protocol::{ClientId, EntityId, NetVec2, NetworkMessage, Topic, WorldSnapshot, WorldUpdate};
use shared::protocol::utils::utils::BinaryEncode;
use crate::config::ServerConfig;
use crate::net::area_of_interest::{is_inside_area_of_interest, DEFAULT_AREA_OF_INTEREST_RADIUS};
use crate::net::network_event::{BrokerShardPeer, SharedEntityRegistry};
use crate::world::{Position};

#[derive(Resource, Default)]
pub struct PublishedEntityPositions {
    positions_by_entity: HashMap<EntityId, Position>,
}

pub fn publish_player_position_updates(
    broker: Res<BrokerShardPeer>,
    shared_registry: Res<SharedEntityRegistry>,
    mut published_positions: ResMut<PublishedEntityPositions>,
    positions: Query<&mut Position>,
) {
    if !broker.is_ready() {
        return;
    }

    match shared_registry.try_lock() {
        Some((.., ent_registry)) => {
            published_positions
                .positions_by_entity
                .retain(|entity_id, _| ent_registry.by_network_id.contains_key(entity_id));

            for (entity_id, entity) in  ent_registry.by_network_id.iter() {

                match positions.get(*entity){
                    Ok(ent_position)=>{
                        let Some(pub_position) = published_positions.positions_by_entity.get(entity_id) else {
                            continue;
                        };

                        if pub_position.0 == ent_position.0 {
                            continue;
                        }

                        let message = NetworkMessage::PositionUpdate {
                            entity_id: *entity_id,
                            position: NetVec2::from_f32(ent_position.0.x, ent_position.0.y,NetVec2::DEFAULT_PRECISION),
                        };

                        if let Err(error) = broker.send_message(&message) {
                            tracing::error!(
                            "failed to publish position update for entity_id={}: {error:#}",
                                entity_id.0
                            );
                            return;
                        }
                        published_positions
                            .positions_by_entity
                            .insert(*entity_id, *ent_position);

                    }
                    Err(_)=>{
                        continue
                    }
                }
            }
        }
        None => {
            tracing::warn!("could not lock player registry for client input");
            return;
        }
    }
}

pub fn publish_world_update(
    broker: Res<BrokerShardPeer>,
    shared_registry: Res<SharedEntityRegistry>,
    config: Res<ServerConfig>,
) {
    if !broker.is_ready() {
        return;
    }

    let Topic::ShardInstance(shard_id) = config.shard_topic else {
        tracing::warn!(
            "cannot publish WorldUpdate to unsupported topic {}",
            config.shard_topic.to_string()
        );
        return;
    };


    match shared_registry.try_lock() {
        Some((cli_registry, ent_registry))=> {
            // Do Something
        }
        None => {
            tracing::warn!("could not lock player registry for client input");
            return;
        }
    }

















    let Ok(registry) = registry.entity_reg_shared.try_lock() else {
        tracing::warn!("could not lock player registry for world update");
        return;
    };

    if(registry.players.is_empty()){
        return;
    }


    let full_players = registry.generate_player_snapshot();

    for observer in &full_players {
        let players = full_players
            .iter()
            .filter(|player| {
                player.client_id == observer.client_id
                    || is_inside_area_of_interest(
                    observer.position,
                    player.position,
                    DEFAULT_AREA_OF_INTEREST_RADIUS,
                )
            })
            .cloned()
            .collect();

        let snapshot = WorldSnapshot {
            zone: config.zone.clone(),
            players,
            server_tick: config.server_tick,
        };

        let update = WorldUpdate::Snapshot { snapshot };

        let mut payload = Vec::new();

        match (update.encode_binary(&mut payload)) {
            Ok(payload) => payload,
            Err(error) => {
                tracing::error!(
                    "failed to encode WorldUpdate for client_id={}: {error:#}",
                    observer.client_id.0
                );
                continue;
            }
        };

        let payload_len = match u16::try_from(payload.len()) {
            Ok(payload_len) => payload_len,
            Err(_) => {
                tracing::error!(
                    "WorldUpdate payload too large for client_id={}: {} bytes",
                    observer.client_id.0,
                    payload.len()
                );
                continue;
            }
        };

        let message = NetworkMessage::Publish {
            shard_id,
            client_id: observer.client_id,
            payload_len,
            payload,
        };

        if let Err(error) = broker.send_message(&message) {
            tracing::error!(
                "failed to publish WorldUpdate for client_id={}: {error:#}",
                observer.client_id.0
            );
            return;
        }
    }
}