use std::hash::{Hash, Hasher};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::protocol::{NetVec2, Username, ZoneId};
use crate::protocol::broker::ClientId;

pub type PlayerId = Uuid;
pub const PLAYER_DEFAULT_MOVE_SPEED: f32 = 5.0;
#[derive(Debug, Clone)]
pub struct Player {
    pub player_id: PlayerId,
    pub username: Username,
    pub zone: ZoneId,
    pub position: NetVec2,
    pub velocity: NetVec2,
    pub movement_speed: f32,
}


impl Player {

    pub fn new(player_id: PlayerId, username: Username, zone: ZoneId) -> Self {
        Player {
            player_id,
            username,
            zone,
            position: NetVec2::ZERO,
            velocity: NetVec2::ZERO,
            movement_speed: PLAYER_DEFAULT_MOVE_SPEED,
        }
    }

    pub fn public_info(&self) -> PlayerPublicInfo {
        PlayerPublicInfo {
            username: self.username.clone(),
        }
    }

    pub fn update_movement(&mut self, delta_seconds: f32) {
        let speed = self.movement_speed * delta_seconds;
        let mut position = self.position.to_f32();
        let velocity = self.velocity.to_f32();
        position.0 += velocity.0 * speed;
        position.1 += velocity.1 * speed;
        self.position = NetVec2::from_f32(position.0, position.1,NetVec2::DEFAULT_PRECISION);
    }
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.player_id == other.player_id
    }
}
impl Eq for Player {}

impl Hash for Player {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.player_id.hash(state);
    }
}


/**
player info sent to the client and other players
**/
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerPublicInfo {
    pub username: Username,
}


/**
snapshot of a player, sent to the client
**/
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub client_id: ClientId,
    pub player_id: PlayerId,
    pub username: Username,
    pub position: NetVec2,
    pub velocity: NetVec2,
}
