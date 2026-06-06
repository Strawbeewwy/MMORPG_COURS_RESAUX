use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct EntityId(pub u32);
pub const ENTITY_ID_LEN: usize = 32;
pub const ENTITY_STATE_LEN: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub fn to_le_bytes(self) -> [u8; 1] {
        (self as u8).to_le_bytes()
    }

    pub fn from_le_bytes(bytes: [u8; 1]) -> Result<Self, &'static str> {
        let value = u8::from_le_bytes(bytes);
        Self::try_from(value)
    }
}

impl TryFrom<u8> for EntityState {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(EntityState::Owned),
            1 => Ok(EntityState::PendingHandoff),
            2 => Ok(EntityState::Ghost),
            _ => Err("Invalid byte value for EntityState"),
        }
    }
}

pub struct Entity {
    pub entity_id: EntityId,
    pub entity_type: EntityType,
    pub entity_state: EntityState,
}