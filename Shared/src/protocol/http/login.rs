use serde::{Deserialize, Serialize};

use crate::protocol::discovery::ServerInfo;

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
