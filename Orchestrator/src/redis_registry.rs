use anyhow::{Context, Result};
use redis::AsyncCommands;
use shared::protocol::Heartbeat;

#[derive(Clone)]
pub struct RedisRegistry {
    client: redis::Client,
}

impl RedisRegistry {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }

    pub async fn update_server(
        &self,
        heartbeat: &Heartbeat,
        ttl_seconds: usize,
    ) -> Result<()> {

        anyhow::ensure!(ttl_seconds > 0, "server TTL seconds must be greater than zero");

        let ttl_seconds = i64::try_from(ttl_seconds)
            .context("server TTL seconds does not fit into i64")?;


        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("failed to connect to Redis")?;

        let key = format!("server:{}", heartbeat.id);
        let status = heartbeat.status();

        let _: () = redis::pipe()
            .atomic()
            .hset(&key, "id", &heartbeat.id)
            .hset(&key, "ip", &heartbeat.ip)
            .hset(&key, "port", heartbeat.port)
            .hset(&key, "zone", &heartbeat.zone)
            .hset(&key, "player_count", heartbeat.player_count)
            .hset(&key, "max_players", heartbeat.max_players)
            .hset(&key, "status", status)
            .expire(&key, ttl_seconds)
            .query_async(&mut connection)
            .await
            .context("failed to HSET/EXPIRE server heartbeat in Redis")?;

        Ok(())
    }

    pub async fn count_available_servers(&self) -> Result<usize> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("failed to connect to Redis")?;

        let keys: Vec<String> = connection
            .keys("server:*")
            .await
            .context("failed to list server keys")?;

        let mut available_count = 0;

        for key in keys {
            let status: Option<String> = connection
                .hget(&key, "status")
                .await
                .with_context(|| format!("failed to HGET status for {key}"))?;

            if status.as_deref() == Some("available") {
                available_count += 1;
            }
        }

        Ok(available_count)
    }
}