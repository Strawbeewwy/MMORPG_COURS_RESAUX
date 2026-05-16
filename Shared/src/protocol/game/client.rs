use crate::protocol::game::Username;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientGameMessage {
    JoinGame {///when the player joins the game
        protocol_version: u16,
        session_token: String,
        username: Username,
    },
    LeaveGame,///when the player leaves the game
    Heartbeat,///
    PlayerInput {
        movement_x: f32,
        movement_y: f32,
    },
}