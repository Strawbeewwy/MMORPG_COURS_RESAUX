pub const GAME_SERVER_ADDRESS: &str = "127.0.0.1:5000";
pub const GATEKEEPER_HTTP_ADDRESS: &str = "127.0.0.1:3000";
pub const GATEKEEPER_HTTP_URL: &str = "http://127.0.0.1:3000";
pub const GATEKEEPER_SERVER_NAME: &str = "localhost";
pub const LAUNCHER_VERSION: &str = "0.1.0";

pub const GAME_MESSAGE_SIZE_LIMIT: usize = 64 * 1024;
pub const GAME_PROTOCOL_VERSION: u16 = 1;


pub const SUPPORTED_PROTOCOL_VERSION: u16 = 1;

pub const DEFAULT_ZONE: &str = "default";

pub const DEFAULT_ORCHESTRATOR_HOST: &str = "127.0.0.1";
pub const DEFAULT_ORCHESTRATOR_PORT: u16 = 7000;

pub const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1:6379";

pub const DEFAULT_HOT_SERVERS_MIN: usize = 1;
pub const DEFAULT_SERVER_TTL_SECONDS: usize = 10;
pub const DEFAULT_SCALER_INTERVAL_SECONDS: u64 = 2;

pub const DEFAULT_FIRST_DS_PORT: u16 = 9000;
pub const DEFAULT_DS_BINARY: &str = "GameServer";

pub const DEFAULT_MAX_PLAYERS: usize = 100;

pub const DEFAULT_HEARTBEAT_BUFFER_SIZE: usize = 1024;

pub const DEFAULT_DEBUG_PASSWORD: &str = "1234";