use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct LoginRequest {
    #[serde(rename = "type")]
    pub message_type: String,
    pub username: String,
    pub password: String,
    pub launcher_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
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
}