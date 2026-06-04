use bevy::platform::collections::HashMap;
use bevy::prelude::{Res, ResMut, Resource};
use shared::protocol::{ClientId, NetVec2, NetworkMessage, Topic, WorldSnapshot, WorldUpdate};
use shared::protocol::utils::utils::BinaryEncode;
use crate::config::ServerConfig;
use crate::net::area_of_interest::{is_inside_area_of_interest, DEFAULT_AREA_OF_INTEREST_RADIUS};
use crate::net::network_event::{BrokerShardPeer, SharedPlayerRegistry};




#[derive(Resource, Default)]
pub struct PublishedPlayerPositions {
    positions_by_client: HashMap<ClientId, NetVec2>,
}

pub fn publish_player_position_updates(
    broker: Res<BrokerShardPeer>,
    registry: Res<SharedPlayerRegistry>,
    mut published_positions: ResMut<PublishedPlayerPositions>,
) {
    if !broker.is_ready() {
        return;
    }

    let Ok(registry) = registry.inner.try_lock() else {
        tracing::warn!("could not lock player registry for position updates");
        return;
    };

    published_positions
        .positions_by_client
        .retain(|client_id, _| registry.client_player.contains_key(client_id));

    for (client_id, player_id) in registry.client_player.iter() {
        let Some(player) = registry.players.get(player_id) else {
            tracing::warn!(
                "cannot publish position update: player not found for client_id={}",
                client_id.0
            );
            continue;
        };

        let position = player.position.clone();

        if published_positions.positions_by_client.get(client_id) == Some(&position) {
            continue;
        }

        let message = NetworkMessage::PositionUpdate {
            client_id: *client_id,
            position: position.clone(),
        };

        if let Err(error) = broker.send_message(&message) {
            tracing::error!(
                "failed to publish position update for client_id={}: {error:#}",
                client_id.0
            );
            return;
        }

        published_positions
            .positions_by_client
            .insert(*client_id, position);
    }
}

pub fn publish_world_update(
    broker: Res<BrokerShardPeer>,
    registry: Res<SharedPlayerRegistry>,
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

    let Ok(registry) = registry.inner.try_lock() else {
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