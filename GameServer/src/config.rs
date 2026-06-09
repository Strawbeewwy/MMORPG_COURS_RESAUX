use bevy::prelude::Resource;
use shared::config::{
    DEFAULT_DS_IP, DEFAULT_FIRST_DS_PORT, DEFAULT_MAX_PLAYERS, DEFAULT_ORCHESTRATOR_HOST,
    DEFAULT_ORCHESTRATOR_PORT, DEFAULT_ZONE,
};
use shared::protocol::{ShardId, Topic};
use std::env;
use std::net::SocketAddr;
use anyhow::Context;
use shared::protocol::ZoneId;

pub const DEFAULT_BROKER_IP: &str = "127.0.0.1";
pub const DEFAULT_BROKER_PORT: u16 = 7000;

#[derive(Debug, Clone, Resource)]
pub struct ServerConfig {
    pub ip: String,
    pub port: u16,
    pub zone: ZoneId,
    pub max_players: usize,
    pub orchestrator_addr: SocketAddr,
    pub broker_ip: String,
    pub broker_port: u16,
    pub shard_topic: Topic,
    pub server_tick: u64,
}

impl ServerConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let shard_id = env::var("SHARD_ID")
            .context("missing SHARD_ID env var")?
            .parse::<u32>()
            .context("invalid SHARD_ID env var");

        let ip = env::var("SHARD_IP").unwrap_or_else(|_| DEFAULT_DS_IP.to_string());

        let port = env::var("SHARD_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_FIRST_DS_PORT);

        let zone = ZoneId::from(
            env::var("ZONE").unwrap_or_else(|_| DEFAULT_ZONE.to_string())
        );

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


        let shard_topic = Topic::ShardInstance{
            id: ShardId(shard_id?)
        };

        Ok(Self {
            ip,
            port,
            zone,
            max_players,
            orchestrator_addr,
            broker_ip,
            broker_port,
            shard_topic,
            server_tick: 0,
        })
    }


    pub fn broker_addr(&self) -> String {
        format!("{}:{}", self.broker_ip, self.broker_port)
    }
}