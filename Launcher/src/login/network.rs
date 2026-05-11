use anyhow::{anyhow, Context, Result};
use quinn::{ClientConfig, Endpoint};
use rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use std::net::SocketAddr;
use std::sync::Arc;

use crate::config::{
    GATEKEEPER_ADDRESS, GATEKEEPER_SERVER_NAME, LAUNCHER_VERSION,
    LOGIN_RESPONSE_SIZE_LIMIT,
};
use crate::protocol::{LoginRequest, LoginResponse};

pub async fn login_to_gatekeeper(
    username: &str,
    password: &str,
) -> Result<LoginResponse> {
    let client_address: SocketAddr = "0.0.0.0:0"
        .parse()
        .context("invalid client bind address")?;

    let gatekeeper_address: SocketAddr = GATEKEEPER_ADDRESS
        .parse()
        .context("invalid gatekeeper address")?;

    let mut endpoint = Endpoint::client(client_address)
        .context("failed to create QUIC client endpoint")?;

    endpoint.set_default_client_config(create_insecure_client_config()?);

    let connection = endpoint
        .connect(gatekeeper_address, GATEKEEPER_SERVER_NAME)
        .context("failed to start QUIC connection")?
        .await
        .context("failed to connect to GateKeeper")?;

    let (mut send_stream, mut receive_stream) = connection
        .open_bi()
        .await
        .context("failed to open bidirectional QUIC stream")?;

    let login_request = LoginRequest {
        message_type: "login".to_string(),
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

fn create_insecure_client_config() -> Result<ClientConfig> {
    let mut crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    crypto.alpn_protocols = vec![b"mmorpg-gatekeeper".to_vec()];

    Ok(ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?,
    )))
}

#[derive(Debug)]
struct SkipServerVerification;

impl ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _certificate: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _certificate: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::ED25519,
        ]
    }
}