pub const GATEKEEPER_ADDRESS: &str = "127.0.0.1:4000";
pub const GAME_SERVER_ADDRESS: &str = "127.0.0.1:5000";

pub const GATEKEEPER_SERVER_NAME: &str = "localhost";

pub const GATEKEEPER_ALPN_PROTOCOL: &[u8] = b"mmorpg-gatekeeper";
pub const GAME_SERVER_ALPN_PROTOCOL: &[u8] = b"mmorpg-game-server";

pub const LAUNCHER_VERSION: &str = "0.1.0";

pub const LOGIN_REQUEST_SIZE_LIMIT: usize = 16 * 1024;
pub const LOGIN_RESPONSE_SIZE_LIMIT: usize = 16 * 1024;
pub const GAME_MESSAGE_SIZE_LIMIT: usize = 64 * 1024;

pub const LOGIN_PROTOCOL_VERSION: u16 = 1;
pub const GAME_PROTOCOL_VERSION: u16 = 1;