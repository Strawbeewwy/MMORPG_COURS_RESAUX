use bevy::prelude::Resource;
use shared::protocol::{EntityId, NetVec2, ShardId};
use shared::EntityType;


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


pub struct SpatialEntityRecord {
    pub entity_id: EntityId,
    pub entity_type: EntityType,
    pub current_shard_id: ShardId,
    pub position: NetVec2,
}