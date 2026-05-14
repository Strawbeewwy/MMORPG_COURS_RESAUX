use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerGameMessage {
    JoinAccepted {
        player_id: u64,
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
    Goodbye,
}