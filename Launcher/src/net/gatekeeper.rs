use shared::config::{
    GATEKEEPER_ADDRESS, GATEKEEPER_SERVER_NAME, LAUNCHER_VERSION,
    LOGIN_RESPONSE_SIZE_LIMIT,LOGIN_PROTOCOL_VERSION,
};
use crate::net::tls::create_insecure_client_config;
use shared::protocol::{LoginRequest, LoginResponse};
use anyhow::{anyhow, Context, Result};
use quinn::Endpoint;
use std::net::SocketAddr;

pub async fn login_to_gatekeeper(
    username: &str,
    password: &str,
) -> Result<LoginResponse> {
    let client_address: SocketAddr = "0.0.0.0:0"
        .parse()
        .context("invalid launcher client bind address")?;

    let gatekeeper_address: SocketAddr = GATEKEEPER_ADDRESS
        .parse()
        .context("invalid GateKeeper address")?;

    let mut endpoint = Endpoint::client(client_address)
        .context("failed to create QUIC client endpoint")?;

    endpoint.set_default_client_config(create_insecure_client_config()?);

    let connection = endpoint
        .connect(gatekeeper_address, GATEKEEPER_SERVER_NAME)
        .context("failed to start QUIC connection to GateKeeper")?
        .await
        .context("failed to connect to GateKeeper")?;

    let (mut send_stream, mut receive_stream) = connection
        .open_bi()
        .await
        .context("failed to open GateKeeper bidirectional stream")?;

    let login_request = LoginRequest::Login {
        protocol_version: LOGIN_PROTOCOL_VERSION,
        username: username.to_string(),
        password: password.to_string(),
        launcher_version: LAUNCHER_VERSION.to_string(),
    };

    let request_body = serde_json::to_vec(&login_request)
        .context("failed to serialize login request")?;

    send_stream
        .write_all(&request_body)
        .await
        .context("failed to send login request")?;

    send_stream
        .finish()
        .context("failed to finish login request stream")?;

    let response_body = receive_stream
        .read_to_end(LOGIN_RESPONSE_SIZE_LIMIT)
        .await
        .context("failed to read login response")?;

    if response_body.is_empty() {
        return Err(anyhow!("empty response received from GateKeeper"));
    }

    let login_response = serde_json::from_slice(&response_body)
        .context("failed to parse login response")?;

    connection.close(0u32.into(), b"login complete");

    Ok(login_response)
}