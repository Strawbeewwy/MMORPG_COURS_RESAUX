use serde::{Deserialize, Serialize};

pub type PlayerId = String;
pub type EntityId = String;
pub type ZoneId = String;
pub type Username = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct NetVec2 {
    pub x: f32,
    pub y: f32,
}

impl NetVec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerPublicInfo {
    pub player_id: PlayerId,
    pub username: Username,
    pub zone: ZoneId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerSnapshot {
    pub player_id: PlayerId,
    pub username: Username,
    pub position: NetVec2,
    pub velocity: NetVec2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldSnapshot {
    pub zone: ZoneId,
    pub players: Vec<PlayerSnapshot>,
    pub server_tick: u64,
}