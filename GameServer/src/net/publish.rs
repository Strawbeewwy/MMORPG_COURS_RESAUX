use bevy::platform::collections::HashMap;
use bevy::prelude::{Query, Res, ResMut, Resource, Vec2};
use shared::protocol::{
    EntityId, NetVec2, NetworkMessage,
};
use crate::config::ServerConfig;
use crate::net::area_of_interest::is_inside_area_of_interest;
use crate::net::network_event::BrokerShardPeer;
use crate::world::{Position, NetworkEntityId};
use crate::world::state::SharedEntityRegistry;

#[derive(Resource, Default)]
pub struct PublishedEntityPositions {
    positions_by_entity: HashMap<EntityId, Position>,
}
impl PublishedEntityPositions {
    pub fn track(&mut self, entity_id: EntityId, position: Position) {
        self.positions_by_entity.insert(entity_id, position);
    }
    pub fn get(&self, entity_id: EntityId) -> Option<&Position> {
        self.positions_by_entity.get(&entity_id)
    }
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

                        if let Err(error) = broker.send_message_to_broker(&message) {
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
    positions: Query<(&NetworkEntityId, &Position)>,
    mut published_positions: ResMut<PublishedEntityPositions>,
) {
    if !broker.is_ready() {
        return;
    }

    let Some((_, ent_registry)) = shared_registry.try_lock() else {
        return;
    };

    if ent_registry.by_network_id.is_empty() {
        return;
    }

    published_positions
        .positions_by_entity
        .retain(|entity_id, _| ent_registry.by_network_id.contains_key(entity_id));

    let mut entity_positions = HashMap::new();
    for (net_id, position) in positions.iter() {
        entity_positions.insert(net_id.0, position.0);
    }

    for (observer_entity_id, observer_bevy_entity) in ent_registry.by_network_id.iter() {
        let Some(&observer_position) = entity_positions.get(observer_entity_id) else {
            continue;
        };

        for (entity_id, position) in entity_positions.iter() {
            let is_self = *entity_id == *observer_entity_id;
            let in_aoi = is_self || is_inside_area_of_interest(
                observer_position,
                *position,
                config.aoi_radius,
            );

            if !in_aoi {
                continue;
            }

            let Some(published_pos) = published_positions.get(*entity_id) else {
                continue;
            };

            if published_pos.0 == *position {
                continue;
            }

            let message = NetworkMessage::PositionUpdate {
                entity_id: *entity_id,
                position: NetVec2::from_f32(position.x, position.y, NetVec2::DEFAULT_PRECISION),
            };

            if let Err(error) = broker.send_message_to_broker(&message) {
                tracing::error!(
                    "failed to publish position update for entity_id={}: {error:#}",
                    entity_id.0
                );
                return;
            }

            published_positions.track(*entity_id, Position(*position));
        }
    }
}