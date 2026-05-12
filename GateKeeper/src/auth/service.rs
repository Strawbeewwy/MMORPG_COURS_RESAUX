use crate::auth::session::create_session_token;
use crate::config::{ACCEPTED_PASSWORD, ACCEPTED_USERNAME};
use shared::config::{
    GAME_SERVER_ADDRESS, LAUNCHER_VERSION, LOGIN_PROTOCOL_VERSION,
};
use shared::protocol::{LoginRequest, LoginResponse};

pub fn authenticate(login_request: LoginRequest) -> LoginResponse {
    match login_request {
        LoginRequest::Login {
            protocol_version,
            username,
            password,
            launcher_version,
        } => authenticate_login(protocol_version, username, password, launcher_version),
        LoginRequest::Logout => LoginResponse::Failed {
            reason: "Already logged out".to_string(),
        },
        LoginRequest::Heartbeat => LoginResponse::Failed {
            reason: "Heartbeat is not expected during login".to_string(),
        },
    }
}

fn authenticate_login(
    protocol_version: u16,
    username: String,
    password: String,
    launcher_version: String,
) -> LoginResponse {
    if protocol_version != LOGIN_PROTOCOL_VERSION {
        return LoginResponse::Failed {
            reason: "Unsupported login protocol version.".to_string(),
        };
    }

    if launcher_version != LAUNCHER_VERSION {
        return LoginResponse::Failed {
            reason: "Outdated launcher. Please update.".to_string(),
        };
    }

    if username == ACCEPTED_USERNAME && password == ACCEPTED_PASSWORD {
        return LoginResponse::Success {
            session_token: create_session_token(&username),
            game_server_address: GAME_SERVER_ADDRESS.to_string(),
        };
    }

    LoginResponse::Failed {
        reason: "Invalid username or password".to_string(),
    }
}