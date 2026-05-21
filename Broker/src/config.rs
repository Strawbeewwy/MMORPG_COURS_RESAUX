use bevy::prelude::*;

pub const DEFAULT_BROKER_PORT: u16 = 7000;

#[derive(Resource, Debug, Clone)]
pub struct BrokerConfig {
    pub port: u16,
}

impl BrokerConfig {
    pub fn from_env() -> Self {
        Self {
            port: std::env::var("BROKER_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(DEFAULT_BROKER_PORT),
        }
    }
}