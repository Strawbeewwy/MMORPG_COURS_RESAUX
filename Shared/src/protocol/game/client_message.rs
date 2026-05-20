use crate::protocol::game::Username;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientGameMessage {
    JoinGame {///when the player joins the game
        protocol_version: String,
        session_token: String,
        username: Username,
    },
    LeaveGame,///when the player leaves the game
    Heartbeat,///can be used later to detect if the player is AFK
    PlayerInput {
        /**
        for the movement we don't need x- or y- since
        we can just invert the x and y values depending on
        which key is pressed
        **/
        movement_x: f32,
        movement_y: f32,
    },
}