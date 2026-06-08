use bevy::prelude::*;
use shared::protocol::{
    ClientId,
    EntityId,
    EntityType,
    NetVec2,
    ShardId,
    Username,
};
use crate::world::{Authoritative, ClientEntityRegistry, ControlledByClient, EntityIdAllocator, EntityKind, EntityRegistry, Ghost, NetworkEntityId, Position, Velocity};

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
    pub entity_id: EntityId,
    pub entity_type: EntityType,
    pub position: Vec2,
}


pub fn spawn_player_entities(
    mut commands: Commands,
    mut message: MessageReader<SpawnPlayerEntityEvent>,
    mut allocator: ResMut<EntityIdAllocator>,
    mut entity_index: ResMut<EntityRegistry>,
    mut client_index: ResMut<ClientEntityRegistry>,
) {
    for event in message.read() {
        let Some(entity_id) = allocator.allocate() else {
            warn!(
                "cannot spawn player for client_id={}: no EntityId available",
                event.client_id.0
            );
            continue;
        };

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

        info!(
            "spawned player entity_id={} client_id={}",
            entity_id.0,
            event.client_id.0
        );
    }
}

pub fn spawn_ghost_entities(
    mut commands: Commands,
    mut message: MessageReader<SpawnGhostEntityEvent>,
    mut entity_index: ResMut<EntityRegistry>,
) {
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

pub fn spawn_generic_entities(
    mut commands: Commands,
    mut message: MessageReader<SpawnGenericEntityEvent>,
    mut entity_index: ResMut<EntityRegistry>,
) {
    for event in message.read() {
        if entity_index.get_bevy_entity(&event.entity_id).is_some() {
            continue;
        }

        let bevy_entity = commands
            .spawn((
                NetworkEntityId(event.entity_id),
                EntityKind(event.entity_type),
                Position(event.position),
            ))
            .id();

        entity_index.insert(event.entity_id, bevy_entity);

        info!(
            "spawned generic entity_id={}",
            event.entity_id.0,
        );
    }
}