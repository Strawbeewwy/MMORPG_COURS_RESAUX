use anyhow::{Context, Result};
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use shared::protocol::ServerInfo;

pub fn create_pool(redis_url: &str) -> Result<Pool> {
    let config = Config::from_url(redis_url);

    config
        .create_pool(Some(Runtime::Tokio1))
        .context("failed to create Redis connection pool")
}

pub async fn find_available_server(pool: &Pool) -> Result<Option<ServerInfo>> {
    let mut connection = pool
        .get()
        .await
        .context("failed to get Redis connection from pool")?;

    let server_keys: Vec<String> = connection
        .keys("server:*")
        .await
        .context("failed to list server keys from Redis")?;

    for key in server_keys {
        let status: Option<String> = connection
            .hget(&key, "status")
            .await
            .with_context(|| format!("failed to read status for Redis key {key}"))?;

        if status.as_deref() != Some("available") {
            continue;
        }

        let ip: String = connection
            .hget(&key, "ip")
            .await
            .with_context(|| format!("failed to read ip for Redis key {key}"))?;

        let port: u16 = connection
            .hget(&key, "port")
            .await
            .with_context(|| format!("failed to read port for Redis key {key}"))?;

        let zone: String = connection
            .hget(&key, "zone")
            .await
            .with_context(|| format!("failed to read zone for Redis key {key}"))?;

        return Ok(Some(ServerInfo { ip, port, zone }));
    }

    Ok(None)
}