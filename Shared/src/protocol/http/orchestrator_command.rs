
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OrchestratorCommand {
    SpawnServer {
        count: u16,
        reason: String,
    },
    SpawnShardServers {
        shard_ids: Vec<u32>,
        reason: String,
    },
    StopShardServers {
        shard_ids: Vec<u32>,
        reason: String,
    },
    SpatialHello{
        spatial_info: String,
    }
}

