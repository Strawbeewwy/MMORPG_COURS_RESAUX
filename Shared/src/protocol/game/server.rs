use crate::protocol::game::{PlayerId, PlayerPublicInfo, WorldSnapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerGameMessage {
    JoinAccepted {
        player_id: PlayerId,
        player: PlayerPublicInfo,
        snapshot: WorldSnapshot,
        message: String,
    },
    JoinRejected {
        reason: String,
    },
    HeartbeatAck,
    InputAccepted {
        movement_x: f32,
        movement_y: f32,
    },
    WorldSnapshot {
        snapshot: WorldSnapshot,
    },
    PlayerJoined {
        player: PlayerPublicInfo,
    },
    PlayerLeft {
        player_id: PlayerId,
    },
    Goodbye,
}