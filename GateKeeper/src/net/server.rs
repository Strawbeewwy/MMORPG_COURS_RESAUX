use crate::config::GATEKEEPER_BIND_ADDRESS;
use crate::net::certificate::generate_self_signed_certificate;
use crate::net::stream;
use anyhow::{Context, Result};
use quinn::{Endpoint, ServerConfig};
use shared::config::GATEKEEPER_ALPN_PROTOCOL;
use std::net::SocketAddr;
use std::sync::Arc;

pub async fn run() -> Result<()> {
    let bind_address: SocketAddr = GATEKEEPER_BIND_ADDRESS
        .parse()
        .context("invalid GateKeeper bind address")?;

    let server_config = create_server_config()
        .context("failed to create GateKeeper server config")?;

    let endpoint = Endpoint::server(server_config, bind_address)
        .context("failed to start GateKeeper QUIC endpoint")?;

    println!("GateKeeper listening on {bind_address}");

    while let Some(incoming) = endpoint.accept().await {
        tokio::spawn(async move {
            if let Err(error) = stream::handle_connection(incoming).await {
                eprintln!("GateKeeper connection error: {error:?}");
            }
        });
    }

    Ok(())
}

fn create_server_config() -> Result<ServerConfig> {
    let certified_key = generate_self_signed_certificate()
        .context("failed to generate GateKeeper certificate")?;

    let mut crypto = rustls::ServerConfig::builder_with_provider(
        rustls::crypto::aws_lc_rs::default_provider().into(),
    )
        .with_protocol_versions(&[&rustls::version::TLS13])
        .context("failed to configure GateKeeper TLS protocol versions")?
        .with_no_client_auth()
        .with_single_cert(certified_key.cert_chain, certified_key.key_der)
        .context("failed to configure GateKeeper certificate")?;

    crypto.alpn_protocols = vec![GATEKEEPER_ALPN_PROTOCOL.to_vec()];

    Ok(ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(crypto)?,
    )))
}