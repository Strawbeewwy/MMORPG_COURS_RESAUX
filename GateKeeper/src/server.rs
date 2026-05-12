
use anyhow::{Context, Result};
use quinn::{Endpoint, ServerConfig};
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::net::SocketAddr;
use std::sync::Arc;

use crate::config::{
    ACCEPTED_PASSWORD, ACCEPTED_USERNAME, ALPN_PROTOCOL, GAME_SERVER_ADDRESS,
    GATEKEEPER_BIND_ADDRESS, LOGIN_REQUEST_SIZE_LIMIT,
};
use crate::protocol::{LoginRequest, LoginResponse};

pub async fn run() -> Result<()> {
    let bind_address: SocketAddr = GATEKEEPER_BIND_ADDRESS
        .parse()
        .context("invalid GateKeeper bind address")?;

    let server_config = create_server_config()
        .context("failed to create GateKeeper server config")?;

    let endpoint = Endpoint::server(server_config, bind_address)
        .context("failed to start GateKeeper QUIC endpoint")?;

    println!("GateKeeper listening on {bind_address}");

    while let Some(connecting) = endpoint.accept().await {
        tokio::spawn(async move {
            if let Err(error) = handle_connection(connecting).await {
                eprintln!("connection error: {error:?}");
            }
        });
    }

    Ok(())
}

async fn handle_connection(connecting: quinn::Incoming) -> Result<()> {
    let connection = connecting
        .await
        .context("failed to establish incoming QUIC connection")?;

    println!("Client connected: {}", connection.remote_address());

    while let Ok((send_stream, receive_stream)) = connection.accept_bi().await {
        tokio::spawn(async move {
            if let Err(error) = handle_login_stream(send_stream, receive_stream).await {
                eprintln!("stream error: {error:?}");
            }
        });
    }

    println!("Client disconnected");

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

    let login_response = authenticate(login_request);

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

fn authenticate(login_request: LoginRequest) -> LoginResponse {
    if login_request.message_type != "login" {
        return LoginResponse::Failed {
            reason: "Invalid request type".to_string(),
        };
    }

    if login_request.username == ACCEPTED_USERNAME
        && login_request.password == ACCEPTED_PASSWORD
    {
        LoginResponse::Success {
            session_token: create_fake_session_token(&login_request.username),
            game_server_address: GAME_SERVER_ADDRESS.to_string(),
        }
    } else {
        LoginResponse::Failed {
            reason: "Invalid username or password".to_string(),
        }
    }
}

fn create_fake_session_token(username: &str) -> String {
    format!("dev-session-token-for-{username}")
}

fn create_server_config() -> Result<ServerConfig> {
    let certified_key = generate_self_signed_certificate()
        .context("failed to generate self-signed certificate")?;

    let mut crypto = rustls::ServerConfig::builder_with_provider(
        rustls::crypto::aws_lc_rs::default_provider().into(),
    )
        .with_protocol_versions(&[&rustls::version::TLS13])
        .context("failed to configure rustls protocol versions")?
        .with_no_client_auth()
        .with_single_cert(certified_key.cert_chain, certified_key.key_der)
        .context("failed to configure GateKeeper certificate")?;

    crypto.alpn_protocols = vec![ALPN_PROTOCOL.to_vec()];

    Ok(ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(crypto)?,
    )))
}

struct CertifiedKey {
    cert_chain: Vec<CertificateDer<'static>>,
    key_der: PrivateKeyDer<'static>,
}

fn generate_self_signed_certificate() -> Result<CertifiedKey> {
    let cert = generate_simple_self_signed(vec!["localhost".to_string()])
        .context("failed to generate self-signed certificate")?;

    let cert_der = CertificateDer::from(cert.cert.der().to_vec());

    let key_der = PrivateKeyDer::from(
        PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der()),
    );

    Ok(CertifiedKey {
        cert_chain: vec![cert_der],
        key_der,
    })
}