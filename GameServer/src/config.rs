use bevy::prelude::Resource;
use std::env;
use std::net::{SocketAddr};
use uuid::Uuid;
use shared::config::{DEFAULT_ORCHESTRATOR_PORT, DEFAULT_ORCHESTRATOR_HOST,
DEFAULT_MAX_PLAYERS,DEFAULT_ZONE,DEFAULT_FIRST_DS_PORT,DEFAULT_DS_IP,
};

#[derive(Debug, Clone, Resource)]
pub struct ServerConfig {
    pub id: String,
    pub ip: String,
    pub port: u16,
    pub zone: String,
    pub max_players: usize,
    pub orchestrator_addr: SocketAddr,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let id = env::var("DS_ID").unwrap_or_else(|_| Uuid::new_v4().to_string());

        let ip = env::var("DS_IP").unwrap_or_else(|_| DEFAULT_DS_IP.to_string());

        let port = env::var("DS_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_FIRST_DS_PORT);

        let zone = env::var("ZONE").unwrap_or_else(|_| DEFAULT_ZONE.to_string());

        let max_players = env::var("MAX_PLAYERS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_MAX_PLAYERS);

        let orchestrator_addr = env::var("ORCH_ADDR")
            .unwrap_or_else(|_| format!("{DEFAULT_ORCHESTRATOR_HOST}:{DEFAULT_ORCHESTRATOR_PORT}"))
            .parse()
            .expect("invalid ORCH_ADDR");

        Self {
            id,
            ip,
            port,
            zone,
            max_players,
            orchestrator_addr,
        }
    }
}