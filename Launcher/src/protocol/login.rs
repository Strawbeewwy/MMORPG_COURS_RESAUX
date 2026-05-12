use serde::{Deserialize, Serialize};

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