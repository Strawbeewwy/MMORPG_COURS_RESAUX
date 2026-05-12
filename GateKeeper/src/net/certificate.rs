use anyhow::{Context, Result};
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

pub struct CertifiedKey {
    pub cert_chain: Vec<CertificateDer<'static>>,
    pub key_der: PrivateKeyDer<'static>,
}

pub fn generate_self_signed_certificate() -> Result<CertifiedKey> {
    let cert = generate_simple_self_signed(vec!["localhost".to_string()])
        .context("failed to generate self-signed GateKeeper certificate")?;

    let cert_der = CertificateDer::from(cert.cert.der().to_vec());

    let key_der = PrivateKeyDer::from(
        PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der()),
    );

    Ok(CertifiedKey {
        cert_chain: vec![cert_der],
        key_der,
    })
}