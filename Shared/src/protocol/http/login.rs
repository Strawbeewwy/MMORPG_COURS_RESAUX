use serde::{Deserialize, Serialize};

use crate::protocol::discovery::ServerInfo;


/*
request to login to the game
*/
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginHttpRequest {
    pub username: String,
    pub password: String,
}

/*
response to the login request
*/
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginHttpResponse {
    pub client_id: u32,
}
