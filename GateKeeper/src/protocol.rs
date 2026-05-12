/**
protocol.rs contains the responses and requests that are sent
between the client and the server. They are Json-serialized
for debugging purposes. Later we can ditch the
Json-serialization, since, in a real-world application,
we would use a binary protocol for efficiency. Although,
unlike a player transform in a game, the systems request and
response are not sent frequently. We still have to think that
10 000 players may try to log in at the same time.
**/

use serde::{Deserialize, Serialize};
/// Messages sent from the Client to the Gatekeeper
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LoginRequest {
    Login {
        username: String,
        password: String,
        launcher_version: String,
    },
    Logout,
    Heartbeat,
}

/// Messages sent from the Gatekeeper to the Client
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LoginResponse {
    #[serde(rename = "login_success")]
    Success {
        session_token: String,
        game_server_address: String,
    },
    #[serde(rename = "login_failed")]
    Failed {
        reason: String,
    },
    ServerFull {
        queue_position: u32,
    },
}