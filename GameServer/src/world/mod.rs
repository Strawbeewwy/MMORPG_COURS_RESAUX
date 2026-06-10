
pub mod entity;
pub mod state;
pub mod spawn_entity;
pub mod enemy;
pub mod projectile;
pub mod combat;

pub use entity::{
    Authoritative,
    ControlledByClient,
    EntityKind,
    Ghost,
    NetworkEntityId,
    PendingHandoff,
    Position,
    Velocity,
};

pub use state::{
    ClientEntityRegistry,
    EntityIdAllocator,
    EntityIdRange,
    EntityRegistry,
    SharedEntityRegistry,
};

pub use spawn_entity::{
    SpawnGhostEntityEvent,
    SpawnGenericEntityEvent,
    SpawnPlayerEntityEvent,
};

pub use entity::PromoteGhostEvent;

