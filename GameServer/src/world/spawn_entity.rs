use bevy::prelude::*;
use shared::{encode_message, NetVec2, NetworkMessage};
use shared::protocol::{
    ClientId,
    EntityId,
    EntityType,
    ShardId,
    Username,
};
use crate::config::ServerConfig;
use crate::net::network_event::BrokerShardPeer;
use crate::net::publish::PublishedEntityPositions;
use crate::world::{Authoritative, ControlledByClient, EntityIdAllocator, EntityKind, Ghost, NetworkEntityId, Position, SharedEntityRegistry, Velocity};

#[derive(Message, Debug, Clone)]
pub struct SpawnPlayerEntityEvent {
    pub client_id: ClientId,
    pub username: Username,
    pub position: Vec2,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct SpawnGhostEntityEvent {
    pub entity_id: EntityId,
    pub entity_type: EntityType,
    pub source_shard_id: ShardId,
    pub position: Vec2,
    pub velocity: Vec2,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct SpawnGenericEntityEvent {
    pub entity_type: EntityType,
    pub position: Vec2,
}


pub fn spawn_player_entities(
    mut commands: Commands,
    mut message: MessageReader<SpawnPlayerEntityEvent>,
    mut allocator: ResMut<EntityIdAllocator>,
    shared_registry: ResMut<SharedEntityRegistry>,
    mut published_positions: ResMut<PublishedEntityPositions>,
    mut broker: ResMut<BrokerShardPeer>,
    mut config: ResMut<ServerConfig>,
) {

    match shared_registry.try_lock() {
        Some((mut client_index, mut entity_index))=> {

            for event in message.read() {
                let Some(entity_id) = allocator.allocate() else {
                    warn!(
                    "cannot spawn player for client_id={}: no EntityId available",
                    event.client_id.0
                    );
                    continue;
                };

                if client_index.client_to_entity.contains_key(&event.client_id) {
                    warn!(
                    "client_id={} already has a spawned entity",
                    event.client_id.0
                    );
                    continue;
                }

                let bevy_entity = commands
                    .spawn((
                        NetworkEntityId(entity_id),
                        EntityKind(EntityType::Player),
                        Position(event.position),
                        Velocity(Vec2::ZERO),
                        Authoritative,
                        ControlledByClient {
                            client_id: event.client_id,
                        },
                    ))
                    .id();

                entity_index.insert(entity_id, bevy_entity);
                client_index.insert(event.client_id, entity_id);
                published_positions.track(entity_id, Position(event.position));

                info!(
                "spawned player entity_id={} client_id={}",
                entity_id.0,
                event.client_id.0
                 );

                register_entity_to_spatial(entity_id,&mut published_positions, &mut broker,&mut config);
            }
        }
        None => {
            tracing::warn!("could not lock player registry for client input");
            return;
        }
    }
}

pub fn spawn_ghost_entities(
    mut commands: Commands,
    mut message: MessageReader<SpawnGhostEntityEvent>,
    mut allocator: ResMut<EntityIdAllocator>,
    shared_registry: ResMut<SharedEntityRegistry>,
    mut published_positions: ResMut<PublishedEntityPositions>,
    mut broker: ResMut<BrokerShardPeer>,
    mut config: ResMut<ServerConfig>,
) {

    match shared_registry.try_lock() {
        Some((.., mut entity_index))=> {
            for event in message.read() {
                if entity_index.get_bevy_entity(&event.entity_id).is_some() {
                    continue;
                }

                let bevy_entity = commands
                    .spawn((
                        NetworkEntityId(event.entity_id),
                        EntityKind(event.entity_type),
                        Position(event.position),
                        Velocity(event.velocity),
                        Ghost {
                            source_shard_id: event.source_shard_id,
                        },
                    ))
                    .id();

                entity_index.insert(event.entity_id, bevy_entity);

                info!(
                "spawned ghost entity_id={} from_shard_id={}",
                event.entity_id.0,
                event.source_shard_id.0
                 );
            }
        }
        None => {
            tracing::warn!("could not lock player registry for client input");
            return;
        }
    }
}

pub fn spawn_generic_entities(
    mut commands: Commands,
    mut message: MessageReader<SpawnGenericEntityEvent>,
    mut allocator: ResMut<EntityIdAllocator>,
    shared_registry: ResMut<SharedEntityRegistry>,
    mut published_positions: ResMut<PublishedEntityPositions>,
    mut broker: ResMut<BrokerShardPeer>,
    mut config: ResMut<ServerConfig>,
) {

    match shared_registry.try_lock() {
        Some((.., mut entity_index))=> {
            for event in message.read() {
                let Some(entity_id) = allocator.allocate() else {
                    warn!(
                    "cannot spawn entity: no EntityId available",
                    );
                    continue;
                };



                if entity_index.get_bevy_entity(&entity_id).is_some() {
                    continue;
                }

                let bevy_entity = commands
                    .spawn((
                        NetworkEntityId(entity_id),
                        EntityKind(event.entity_type),
                        Position(event.position),
                    ))
                    .id();

                entity_index.insert(entity_id, bevy_entity);

                info!(
                "spawned generic entity_id={}",
                 entity_id.0,
                );

                register_entity_to_spatial(entity_id,&mut published_positions, &mut broker,&mut config);

            }
        }
        None => {
            tracing::warn!("could not lock player registry for client input");
            return;
        }
    }
}

fn register_entity_to_spatial(
    entity_id: EntityId,
    published_positions: &mut PublishedEntityPositions,
    broker: &mut BrokerShardPeer,
    config: &mut ServerConfig,
) {

    let found_position = match published_positions.get(entity_id) {
        Some(position) => position,
        None => return,
    };

    let network_position = NetVec2::from_f32(
        found_position.0.x,
        found_position.0.y,
        NetVec2::DEFAULT_PRECISION
    );

    let message = &NetworkMessage::RegisterEntity {
        entity_id,
        position: network_position,
    };

    if let Err(error) = broker.send_message_to_broker(message) {
        tracing::error!("failed to send packet to broker: {error:#}");
        return;
    }


    info!(
        "registered entity with spatial id={}",
        entity_id.0
    );


}