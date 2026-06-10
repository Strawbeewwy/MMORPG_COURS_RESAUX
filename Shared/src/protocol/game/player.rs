use std::hash::{Hash, Hasher};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::protocol::{NetVec2, ZoneId, ClientId};
use crate::protocol::utils::utils::{read_username, write_username, BinaryDecode, BinaryEncode};

pub type Username = Arc<str>;

pub type PlayerId = u128;
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


impl BinaryEncode for PlayerPublicInfo {
    fn encode_binary(&self, output: &mut Vec<u8>) -> anyhow::Result<()> {
        write_username(output, &self.username)
    }
}

impl BinaryDecode for PlayerPublicInfo {
    fn decode_binary(input: &mut &[u8]) -> anyhow::Result<Self> {
        let username = read_username(input)?;

        Ok(PlayerPublicInfo {
            username,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSpawnInfo {
    pub username: Username,
    pub zone: ZoneId,
    pub spawn_position: NetVec2,
}



