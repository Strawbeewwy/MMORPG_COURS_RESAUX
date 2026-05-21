use anyhow::{Context, Result};
use bevy::prelude::*;
use shared::protocol::broker::{Topic, topic_from_str};
use std::env;

pub const DEFAULT_RECONNECT_INTERVAL: u64 = 5;
pub const DEFAULT_BROKER_TOPIC: &str = "shard:0";

#[derive(Resource, Debug, Clone)]
pub struct ClientConfig {
    pub player_id: String,
    pub username: String,
    pub broker_ip: String,
    pub broker_port: u16,
    pub zone: String,
    pub broker_topics: Vec<Topic>,
}

impl ClientConfig {
    pub fn from_env() -> Result<Self> {
        let player_id = env::var("PLAYER_ID")
            .context("missing PLAYER_ID env var")?;

        let username = env::var("USERNAME")
            .context("missing USERNAME env var")?;

        let broker_ip = env::var("BROKER_IP")
            .context("missing BROKER_IP env var")?;

        let broker_port = env::var("BROKER_PORT")
            .context("missing BROKER_PORT env var")?
            .parse::<u16>()
            .context("invalid BROKER_PORT or GAME_SERVER_PORT env var")?;

        let zone = env::var("GAME_SERVER_ZONE")
            .context("missing GAME_SERVER_ZONE env var")?;

        let broker_topics = broker_topics_from_env();

        Ok(Self {
            player_id,
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

    pub fn client_id(&self) -> u32 {
        self.player_id
            .parse::<u32>()
            .unwrap_or_else(|_| stable_client_id_from_string(&self.player_id))
    }
}

fn broker_topics_from_env() -> Vec<Topic> {
    let topics = env::var("BROKER_TOPICS")
        .unwrap_or_else(|_| DEFAULT_BROKER_TOPIC.to_string());

    let parsed_topics: Vec<Topic> = topics
        .split(',')
        .map(str::trim)
        .filter(|topic| !topic.is_empty())
        .map(topic_from_str)
        .collect();

    if parsed_topics.is_empty() {
        vec![topic_from_str(DEFAULT_BROKER_TOPIC)]
    } else {
        parsed_topics
    }
}

fn stable_client_id_from_string(value: &str) -> u32 {
    let mut hash = 2_166_136_261_u32;

    for byte in value.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16_777_619);
    }

    hash
}