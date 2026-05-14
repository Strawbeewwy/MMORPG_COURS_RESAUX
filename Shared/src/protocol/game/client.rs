use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientGameMessage {
    JoinGame {
        protocol_version: u16,
        session_token: String,
    },
    LeaveGame,
    Heartbeat,
    PlayerInput {
        movement_x: f32,
        movement_y: f32,
    },
}
