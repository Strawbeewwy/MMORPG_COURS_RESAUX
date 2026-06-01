use anyhow::{Context, Result};
use bevy::prelude::*;
use shared::protocol::{ShardId, Topic};
use std::env;
use std::sync::Arc;
use shared::protocol::{Username, ZoneId};

pub const DEFAULT_RECONNECT_INTERVAL: u64 = 5;

#[derive(Resource, Debug, Clone)]
pub struct ClientConfig {
    pub username: Username,
    pub broker_ip: String,
    pub broker_port: u16,
    pub zone: ZoneId,
    pub broker_topics: Vec<Topic>,
}

impl ClientConfig {
    pub fn from_env() -> Result<Self> {
        let username: Username = Arc::from(env::var("USERNAME")
            .context("missing USERNAME env var")?);
    
        let broker_ip = env::var("BROKER_IP")
            .context("missing BROKER_IP env var")?;

        let broker_port = env::var("BROKER_PORT")
            .context("missing BROKER_PORT env var")?
            .parse::<u16>()
            .context("invalid BROKER_PORT or GAME_SERVER_PORT env var")?;

        let zone = ZoneId::from(
            env::var("GAME_SERVER_ZONE")
            .context("missing GAME_SERVER_ZONE env var")?
        );


        let shard_id = env::var("SHARD_ID")
            .context("missing SHARD_ID env var")?
            .parse::<u32>()
            .context("invalid SHARD_ID env var")?;

        let broker_topics = vec![Topic::ShardInstance(ShardId(shard_id))];

        Ok(Self {
            username,
            broker_ip,
            broker_port,
            zone,
            broker_topics,
        })
    }

    pub fn broker_addr(&self) -> String {
        format!("{}:{}", self.broker_ip, self.broker_port)
    }
}