use serde::{Deserialize, Serialize};

pub type PlayerId = u32;
pub type EntityId = u32;
pub type ZoneId = String;
pub type Username = String;

/**
2D vector sent on the network, not used for math
just for values
**/
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct NetVec2 {
    pub x: f32,
    pub y: f32,
}

impl NetVec2 {
    ///Zero vector
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    ///basic constructor
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/**
player info sent to the client and other players
**/
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerPublicInfo {
    pub player_id: PlayerId,
    pub username: Username,
    pub zone: ZoneId,
}

/**
snapshot of a player, sent to the client
**/
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerSnapshot {
    pub player_id: PlayerId,
    pub username: Username,
    pub position: NetVec2,
    pub velocity: NetVec2,
}

/**
snapshot of the world, sent to the client
**/
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldSnapshot {
    pub zone: ZoneId,
    pub players: Vec<PlayerSnapshot>,
    pub server_tick: u64,
}