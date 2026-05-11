/**
config.rs contains all constants that are used throughout
the application. We use constants to make it easier to change
the configuration of the application. For example, we could
change the address of the gatekeeper server here.
**/

pub const GATEKEEPER_ADDRESS: &str = "127.0.0.1:4000";
pub const GATEKEEPER_SERVER_NAME: &str = "localhost";
pub const LAUNCHER_VERSION: &str = "0.1.0";

pub const LOGIN_RESPONSE_SIZE_LIMIT: usize = 16 * 1024;