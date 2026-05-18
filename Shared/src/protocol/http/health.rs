use serde::{Deserialize, Serialize};

/*
response to the health check from the gatekeeper to the
orchestrator
*/
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}
