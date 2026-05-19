use anyhow::{Context, Result};
use bevy::prelude::*;
use std::env;

pub const DEFAULT_RECONNECT_INTERVAL: u64 = 5;

#[derive(Resource, Debug, Clone)]
pub struct ClientConfig {
    pub player_id: String,
    pub username: String,
    pub server_ip: String,
    pub server_port: u16,
    pub zone: String,
}

impl ClientConfig {
    pub fn from_env() -> Result<Self> {
        let player_id = env::var("PLAYER_ID")
            .context("missing PLAYER_ID env var")?;

        let username = env::var("USERNAME")
            .context("missing USERNAME env var")?;

        let server_ip = env::var("GAME_SERVER_IP")
            .context("missing GAME_SERVER_IP env var")?;

        let server_port = env::var("GAME_SERVER_PORT")
            .context("missing GAME_SERVER_PORT env var")?
            .parse::<u16>()
            .context("invalid GAME_SERVER_PORT env var")?;

        let zone = env::var("GAME_SERVER_ZONE")
            .context("missing GAME_SERVER_ZONE env var")?;

        Ok(Self {
            player_id,
            username,
            server_ip,
            server_port,
            zone,
        })
    }

    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server_ip, self.server_port)
    }
}