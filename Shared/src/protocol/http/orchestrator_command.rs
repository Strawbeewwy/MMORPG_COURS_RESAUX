
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OrchestratorCommand {
    SpawnServer {
        count: u16,
        reason: String,
    },
    SpatialHello{
        spatial_info: String,
    }
}

