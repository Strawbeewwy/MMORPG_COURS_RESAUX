pub const DEFAULT_BROKER_PORT: u16 = 7000;
pub const DEFAULT_BROKER_TICK_MS: u64 = 50;

#[derive(Debug, Clone)]
pub struct BrokerConfig {
    pub port: u16,
    pub tick_ms: u64,
}

impl BrokerConfig {
    pub fn from_env() -> Self {
        Self {
            port: std::env::var("BROKER_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(DEFAULT_BROKER_PORT),

            tick_ms: std::env::var("BROKER_TICK_MS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(DEFAULT_BROKER_TICK_MS),
        }
    }
}