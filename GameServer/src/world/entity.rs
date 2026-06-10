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

/// Marks an authoritative entity that is mid-handoff to `destination`.
/// Removed when HandoffAccepted/HandoffRejected arrives.
#[derive(Component, Debug, Clone, Copy)]
pub struct PendingHandoff {
    pub destination: ShardId,
}

/// Bevy message: dest shard received HandoffCompleted → promote ghost to authoritative.
#[derive(Message, Debug, Clone, Copy)]
pub struct PromoteGhostEvent {
    pub entity_id: EntityId,
}
