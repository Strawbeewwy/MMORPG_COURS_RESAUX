use bevy::prelude::*;
use shared::protocol::{
    ClientId,
    EntityId,
    EntityType,
    ShardId,
};

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkEntityId(pub EntityId);

#[derive(Component, Debug, Clone, Copy)]
pub struct EntityKind(pub EntityType);

#[derive(Component, Debug, Clone, Copy)]
pub struct Position(pub Vec2);

#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity(pub Vec2);

#[derive(Component, Debug, Clone, Copy)]
pub struct Authoritative;

#[derive(Component, Debug, Clone, Copy)]
pub struct Ghost {
    pub source_shard_id: ShardId,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ControlledByClient {
    pub client_id: ClientId,
}