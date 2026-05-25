use crate::protocol::game::{
    NetVec2, PlayerId, Username, ZoneId
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSpawnInfo {
    pub player_id: PlayerId,
    pub username: Username,
    pub zone: ZoneId,
    pub spawn_position: NetVec2,
}
