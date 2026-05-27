use bevy::prelude::Resource;
use shared::config::{
    DEFAULT_DS_IP, DEFAULT_FIRST_DS_PORT, DEFAULT_MAX_PLAYERS, DEFAULT_ORCHESTRATOR_HOST,
    DEFAULT_ORCHESTRATOR_PORT, DEFAULT_ZONE,
};
use shared::protocol::broker::{
    Topic, topic_from_str
};
use std::env;
use std::net::SocketAddr;
use uuid::Uuid;

pub const DEFAULT_BROKER_IP: &str = "127.0.0.1";
pub const DEFAULT_BROKER_PORT: u16 = 7000;
pub const DEFAULT_SHARD_TOPIC: &str = "shard:0";

#[derive(Debug, Clone, Resource)]
pub struct ServerConfig {
    pub id: String,
    pub ip: String,
    pub port: u16,
    pub zone: String,
    pub max_players: usize,
    pub orchestrator_addr: SocketAddr,
    pub broker_ip: String,
    pub broker_port: u16,
    pub shard_topic: Topic,
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

        let broker_ip = env::var("BROKER_IP")
            .unwrap_or_else(|_| DEFAULT_BROKER_IP.to_string());

        let broker_port = env::var("BROKER_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_BROKER_PORT);

        let shard_topic = env::var("SHARD_TOPIC")
            .map(|value| topic_from_str(&value))
            .unwrap_or_else(|_| topic_from_str(DEFAULT_SHARD_TOPIC));

        Self {
            id,
            ip,
            port,
            zone,
            max_players,
            orchestrator_addr,
            broker_ip,
            broker_port,
            shard_topic,
        }
    }

    pub fn broker_addr(&self) -> String {
        format!("{}:{}", self.broker_ip, self.broker_port)
    }
}