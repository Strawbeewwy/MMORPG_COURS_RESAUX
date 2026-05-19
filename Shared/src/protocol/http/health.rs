use serde::{Deserialize, Serialize};

/*
response to the health check
*/
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}
