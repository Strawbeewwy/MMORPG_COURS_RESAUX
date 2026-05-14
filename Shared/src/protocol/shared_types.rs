use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Heartbeat {
    pub id: String,
    pub ip: String,
    pub port: u16,
    pub zone: String,
    pub player_count: usize,
    pub max_players: usize,
}

impl Heartbeat {
    pub fn status(&self) -> &'static str {
        if self.player_count >= self.max_players {
            "full"
        } else {
            "available"
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    pub ip: String,
    pub port: u16,
    pub zone: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginHttpRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginHttpResponse {
    pub player_id: String,
    pub server: ServerInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}