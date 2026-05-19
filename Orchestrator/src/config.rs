use anyhow::{Context, Result};
use std::{env, net::SocketAddr};
use shared::config::{
    DEFAULT_FIRST_DS_PORT,
    DEFAULT_HOT_SERVERS_MIN,
    DEFAULT_ORCHESTRATOR_HOST,
    DEFAULT_ORCHESTRATOR_PORT,
    DEFAULT_REDIS_URL,
    DEFAULT_SCALER_INTERVAL_SECONDS,
    DEFAULT_SERVER_TTL_SECONDS,
    DEFAULT_ZONE,
    DEFAULT_DS_BINARY,
};

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub orch_addr: SocketAddr,
    pub redis_url: String,
    pub hot_servers_min: usize,
    pub server_ttl_seconds: usize,
    pub scaler_interval_seconds: u64,
    pub first_ds_port: u16,
    pub zone: String,
    pub ds_binary: String,
}

impl OrchestratorConfig {
    /*
    when launching the orchestrator, we might want to change the
    default values, so we can do that by setting the environment variables
    */
    pub fn from_env() -> Result<Self> {

        let orch_addr = env::var("ORCH_ADDR")
            .unwrap_or_else(|_| format!("{DEFAULT_ORCHESTRATOR_HOST}:{DEFAULT_ORCHESTRATOR_PORT}"))
            .parse()
            .context("invalid ORCH_ADDR")?;

        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());

        let hot_servers_min = env::var("HOT_SERVERS_MIN")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_HOT_SERVERS_MIN);

        let server_ttl_seconds = env::var("SERVER_TTL_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_SERVER_TTL_SECONDS);

        let scaler_interval_seconds = env::var("SCALER_INTERVAL_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_SCALER_INTERVAL_SECONDS);

        let first_ds_port = env::var("FIRST_DS_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_FIRST_DS_PORT);

        let zone = env::var("ZONE").unwrap_or_else(|_| DEFAULT_ZONE.to_string());

        let ds_binary = env::var("DS_BINARY").unwrap_or_else(|_| DEFAULT_DS_BINARY.to_string());

        Ok(Self {
            orch_addr,
            redis_url,
            hot_servers_min,
            server_ttl_seconds,
            scaler_interval_seconds,
            first_ds_port,
            zone,
            ds_binary,
        })
    }
}
