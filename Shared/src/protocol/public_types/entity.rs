use std::hash::{Hash, Hasher};
use serde::{Deserialize, Serialize};
use crate::protocol::{NetVec2, ShardId};
use crate::protocol::game::Player;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct EntityId(pub u32);
pub const ENTITY_ID_LEN: usize = 32;
pub const ENTITY_STATE_LEN: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EntityType {
    Player,
    Enemy,
    Npc,
    Item,
    Projectile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EntityState{
    Owned,
    PendingHandoff,
    Ghost,
}

impl EntityState {
    pub fn to_le_bytes(&self) -> [u8; size_of::<u8>()] {
        (*self as u8).to_le_bytes()
    }

    pub fn from_le_bytes(bytes: [u8; size_of::<u8>()]) -> Option<Self> {
        
        match bytes[0] {
            0 => Some(EntityState::Owned),
            1 => Some(EntityState::PendingHandoff),
            2 => Some(EntityState::Ghost),
            _ => None, // Handles invalid bytes safely
        }
    }

}

#[derive(Debug, Clone, Copy,)]
pub struct EntityRecord {
    pub entity_id: EntityId,
    pub entity_type: EntityType,
    pub position: NetVec2,
    pub velocity: NetVec2,
    pub state: EntityState,
}

impl PartialEq for EntityRecord {
    fn eq(&self, other: &Self) -> bool {
        self.entity_id == other.entity_id
    }
}
impl Eq for EntityRecord {}

impl Hash for EntityRecord {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity_id.hash(state);
    }
}

#[derive(Debug, Clone, Copy,)]
pub struct GhostEntityRecord {
    pub entity_id: EntityId,
    pub entity_type: EntityType,
    pub position: NetVec2,
    pub velocity: NetVec2,
    pub source_shard_id: ShardId,
}

impl PartialEq for GhostEntityRecord {
    fn eq(&self, other: &Self) -> bool {
        self.entity_id == other.entity_id
    }
}
impl Eq for GhostEntityRecord {}

impl Hash for GhostEntityRecord {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity_id.hash(state);
    }
}
