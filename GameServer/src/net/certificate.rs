use anyhow::{Context, Result};
use quinn::ServerConfig;
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::sync::Arc;

pub fn generate_server_config() -> Result<ServerConfig> {
    let cert = generate_simple_self_signed(vec!["localhost".to_string()])
        .context("failed to generate self-signed certificate")?;

    let cert_der = CertificateDer::from(cert.cert.der().to_vec());

    let key_der = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(
        cert.signing_key.serialize_der(),
    ));

    let mut server_config = ServerConfig::with_single_cert(vec![cert_der], key_der)
        .context("failed to create QUIC server config")?;

    Arc::get_mut(&mut server_config.transport)
        .expect("transport config should not be shared yet")
        .max_concurrent_bidi_streams(64_u32.into());

    Ok(server_config)
}