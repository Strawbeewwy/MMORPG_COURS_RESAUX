use crate::auth::service::authenticate;
use crate::config::LOGIN_REQUEST_SIZE_LIMIT;
use crate::protocol::{LoginRequest, LoginResponse};
use anyhow::{Context, Result};

pub async fn handle_connection(incoming: quinn::Incoming) -> Result<()> {
    let connection = incoming
        .await
        .context("failed to establish incoming GateKeeper QUIC connection")?;

    let remote_address = connection.remote_address();

    println!("Launcher connected to GateKeeper: {remote_address}");

    while let Ok((send_stream, receive_stream)) = connection.accept_bi().await {
        tokio::spawn(async move {
            if let Err(error) = handle_login_stream(send_stream, receive_stream).await {
                eprintln!("GateKeeper stream error: {error:?}");
            }
        });
    }

    println!("Launcher disconnected from GateKeeper: {remote_address}");

    Ok(())
}

async fn handle_login_stream(
    mut send_stream: quinn::SendStream,
    mut receive_stream: quinn::RecvStream,
) -> Result<()> {
    let request_body = receive_stream
        .read_to_end(LOGIN_REQUEST_SIZE_LIMIT)
        .await
        .context("failed to read login request")?;

    let login_request: LoginRequest = serde_json::from_slice(&request_body)
        .context("failed to parse login request")?;

    println!("Login request: {login_request:?}");

    let login_response: LoginResponse = authenticate(login_request);

    let response_body = serde_json::to_vec(&login_response)
        .context("failed to serialize login response")?;

    send_stream
        .write_all(&response_body)
        .await
        .context("failed to send login response")?;

    send_stream
        .finish()
        .context("failed to finish login response stream")?;

    Ok(())
}