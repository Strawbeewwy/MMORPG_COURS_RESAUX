/**
network.rs contains the logic for the QUIC connection to the
gatekeeper.
**/


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



/**
This function sends a systems request to the gatekeeper and
waits for the response. It first creates a client endpoint,
then connects to the gatekeeper, then opens a bidirectional
stream, then sends the systems request, then reads the response,
and finally closes the connection.
**/
pub async fn login_to_gatekeeper(
    username: &str,
    password: &str,
) -> Result<LoginResponse> {

    // bind to a random port
    let client_address: SocketAddr = "0.0.0.0:0"
        .parse()
        .context("invalid client bind address")?;

    //convert the address in config.rs to a SocketAddr
    let gatekeeper_address: SocketAddr = GATEKEEPER_ADDRESS
        .parse()
        .context("invalid gatekeeper address")?;

    // create a client endpoint
    let mut endpoint = Endpoint::client(client_address)
        .context("failed to create QUIC client endpoint")?;

    // set the default client config
    // right now for a local development the certification
    // is not checked, so we use the insecure_client_config
    endpoint.set_default_client_config(create_insecure_client_config()?);


    // connect to the gatekeeper then wait for the handshake with await
    let connection = endpoint
        .connect(gatekeeper_address, GATEKEEPER_SERVER_NAME)
        .context("failed to start QUIC connection")?
        .await
        .context("failed to connect to GateKeeper")?;

    //opens a bidirectional stream
    let (mut send_stream, mut receive_stream) = connection
        .open_bi()
        .await
        .context("failed to open bidirectional QUIC stream")?;

    //create a Login Request
    let login_request = LoginRequest::Login {
        username: username.to_string(),
        password: password.to_string(),
        launcher_version: LAUNCHER_VERSION.to_string(),
        };

    //serialize the Login request
    let request_body = serde_json::to_vec(&login_request)
        .context("failed to serialize systems request")?;

    //send the Login request
    send_stream
        .write_all(&request_body)
        .await
        .context("failed to send systems request")?;

    // finish the stream so the server knows we are done sending
    send_stream
        .finish()
        .context("failed to finish systems request stream")?;

    // read the response
    let response_body = receive_stream
        .read_to_end(LOGIN_RESPONSE_SIZE_LIMIT)
        .await
        .context("failed to read systems response")?;

    if response_body.is_empty() {
        return Err(anyhow!("empty response received from GateKeeper"));
    }
    // deserialize the response
    let login_response = serde_json::from_slice(&response_body)
        .context("failed to parse systems response")?;

    connection.close(0u32.into(), b"systems complete");

    // return the response
    Ok(login_response)
}

fn create_insecure_client_config() -> Result<ClientConfig> {
    let provider = rustls::crypto::aws_lc_rs::default_provider();

    let mut crypto = rustls::ClientConfig::builder_with_provider(provider.into())
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    crypto.alpn_protocols = vec![b"mmorpg-gatekeeper".to_vec()];

    Ok(ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?,
    )))
}


/**
This struct is used to skip the server certificate verification.
We don't need to verify the server certificate since we are
connecting to a local server.
**/
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