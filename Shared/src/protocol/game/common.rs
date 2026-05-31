use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::protocol::broker::{ClientId};
use crate::protocol::{PlayerPublicInfo, PlayerSnapshot};

/// Shared zone identifier — uses `Arc<str>` instead of `String` to avoid repeated
/// heap allocations when the same zone name is cloned across many network messages.
/// Serde serialises/deserialises `Arc<str>` as a plain JSON string transparently.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct EntityId(pub u32);
pub type ZoneId = Arc<str>;
pub type Username = Arc<str>;



/// Spawn information sent by the broker to a shard when placing a new client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSpawnInfo {
    pub username: Username,
    pub zone: ZoneId,
    pub spawn_position: NetVec2,
}

/// World-state update broadcast by the broker to subscribed clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorldUpdate {
    /// Full world snapshot for initial sync or re-sync.
    Snapshot { snapshot: WorldSnapshot },
    /// A new player appeared in the zone.
    PlayerJoined { player: PlayerPublicInfo, client_id: ClientId },
    /// A player left the zone.
    PlayerLeft { player: PlayerPublicInfo , client_id: ClientId},
}

/**
2D vector sent on the network, not used for math
just for values
**/
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq,Eq, Hash)]
pub struct NetVec2 {
    pub x: i32,
    pub y: i32,
    pub precision: u16,// we assume we don't need more than 4 digits of precision
}

impl NetVec2 {

    pub const DEFAULT_PRECISION: u16 = 1000;
    ///Zero vector
    pub const ZERO: Self = Self { x: 0, y: 0 , precision: Self::DEFAULT_PRECISION};

    pub fn from_f32(x: f32, y: f32, precision: u16) -> Self {
        let f32_precision = precision as f32;
        Self {
            x: (x * f32_precision).round() as i32,
            y: (y * f32_precision).round() as i32,
            precision,
        }
    }

    pub fn to_f32(&self) -> (f32,f32) {
        let f32_precision = self.precision as f32;
        (
            self.x as f32 / f32_precision,
            self.y as f32 / f32_precision,
        )
    }

    pub fn to_bytes(&self) -> [u8; 10] {
        let mut bytes = [0u8; 10];
        bytes[0..4].copy_from_slice(&self.x.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.y.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.precision.to_le_bytes());
        bytes
    }
}
impl TryFrom <[u8; 10]> for NetVec2{
    type Error = &'static str;
    fn try_from(bytes: [u8; 10]) -> Result<Self, Self::Error> {
        let x = i32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let y = i32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let precision = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
        Ok(Self { x, y, precision })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    Player,
    Enemy,
    Npc,
    Item,
    Projectile,
    Effect,
}


/**
snapshot of the world, sent to the client
**/
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub zone: ZoneId,
    pub players: Vec<PlayerSnapshot>,
    pub server_tick: u64,
}