use serde::{Deserialize, Serialize};
use crate::protocol::ZoneId;

/**
Heartbeat message sent by the server to the orchestrator.
based on the teacher's example.
**/
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Heartbeat {
    pub id: String,
    pub ip: String,
    pub port: u16,
    pub zone: ZoneId,
    pub player_count: usize,
    pub max_players: u32,
}

/**
Heartbeat message implementation
with a method to get the status of the server.
**/
impl Heartbeat {
    pub fn status(&self) -> &'static str {
        if self.player_count >= self.max_players as usize {
            "full"
        } else {
            "available"
        }
    }
}
