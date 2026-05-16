use serde::{Deserialize, Serialize};

/**
ServerInfo based on the teacher's example.
**/
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    pub ip: String,
    pub port: u16,
    pub zone: String,
}
