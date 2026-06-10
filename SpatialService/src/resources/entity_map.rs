use bevy::platform::collections::HashMap;
use bevy::prelude::{Resource, Vec2};
use shared::protocol::{ClientId, EntityId, NetVec2, ShardId};
use crate::resources::client_map::ClientTransferState;

#[derive(Resource, Debug)]
pub struct GlobalEntityIdAllocator {
    pub next: u32,
}

impl GlobalEntityIdAllocator {
    pub fn allocate_block(&mut self, count: u32) -> Option<(u32, u32)> {
        if count == 0 {
            return None;
        }

        let start = self.next;
        let end = self.next.checked_add(count)?;

        self.next = end;

        Some((start, count))
    }
}

impl Default for GlobalEntityIdAllocator {
    fn default() -> Self {
        Self { next: 1 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityTransferState {
    /// Client is stable on its current shard — handoff may be initiated.
    Stable,
    /// A HandoffRequest has been sent to `destination_shard`; awaiting HandoffAck.
    PendingHandoff { destination_shard: u32 },
}


pub struct SpatialEntityRecord {
    pub entity_id: EntityId,
    /// ClientId(0) means non-player entity.
    pub client_id: ClientId,
    pub position: Vec2,
    pub current_shard: ShardId,
}

impl SpatialEntityRecord {
    pub fn is_player(&self) -> bool {
        self.client_id.0 != 0
    }
}

#[derive(Resource, Default)]
pub struct EntityMap {
    pub entities: HashMap<EntityId,SpatialEntityRecord>,
    client_states: HashMap<EntityId, EntityTransferState>,

}

impl EntityMap{

    pub fn get(&self, entity_id: EntityId) -> Option<&SpatialEntityRecord> {
        self.entities.get(&entity_id)
    }

    pub fn insert(&mut self, entity_id: EntityId, record: SpatialEntityRecord) {
        self.entities.insert(entity_id, record);
    }

    pub fn remove(&mut self, entity_id: EntityId) {
        self.entities.remove(&entity_id);
    }

    pub fn contains(&self, entity_id: EntityId) -> bool {
        self.entities.contains_key(&entity_id)
    }

    // ── Transfer state ─────────────────────────────────────────────────────

    /// Returns `true` if the entity is in `Stable` state (no handoff in progress).
    pub fn is_stable(&self, entity_id: EntityId) -> bool {
        !matches!(
            self.client_states.get(&entity_id),
            Some(EntityTransferState::PendingHandoff { .. })
        )
    }

    /// Mark an entity as pending handoff. Idempotent if called twice for the same destination.
    pub fn set_state(&mut self, entity_id: EntityId, state: EntityTransferState) {
        self.client_states.insert(entity_id, state);
    }

    /// Clear the transfer state (called on Handoff Completed or on client disconnect).
    pub fn clear_state(&mut self, entity_id: EntityId) {
        self.client_states.remove(&entity_id);
    }

    /// Read the transfer state for an entity (absent = Stable).
    pub fn get_state(&self, entity_id: EntityId) -> EntityTransferState {
        self.client_states
            .get(&entity_id)
            .cloned()
            .unwrap_or(EntityTransferState::Stable)
    }
}