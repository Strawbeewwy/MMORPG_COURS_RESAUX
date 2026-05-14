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