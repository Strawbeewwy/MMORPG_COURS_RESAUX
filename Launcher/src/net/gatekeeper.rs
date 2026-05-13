use shared::config::{
    GATEKEEPER_ADDRESS, GATEKEEPER_SERVER_NAME, LAUNCHER_VERSION,
    LOGIN_RESPONSE_SIZE_LIMIT,LOGIN_PROTOCOL_VERSION,
};
use crate::net::tls::create_insecure_client_config;
use shared::protocol::{LoginRequest, LoginResponse};
use anyhow::{Context, Result};
use quinn::Endpoint;
use std::net::SocketAddr;
use shared::protocol::quic_protocol;

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

    let login_request = LoginRequest::Login {
        protocol_version: LOGIN_PROTOCOL_VERSION,
        username: username.to_string(),
        password: password.to_string(),
        launcher_version: LAUNCHER_VERSION.to_string(),
    };

    let login_response = quic_protocol::send_request(
        &connection,
        &login_request,
        LOGIN_RESPONSE_SIZE_LIMIT,
    )
        .await
        .context("failed to exchange login request with GateKeeper")?;

    connection.close(0u32.into(), b"login complete");

    Ok(login_response)
}