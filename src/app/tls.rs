use std::{io, sync::Arc};

use hyperswitch_masking::PeekInterface;
use rustls::{
    ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    server::WebPkiClientVerifier,
};

use crate::config::Config;

pub async fn from_config(config: &Config) -> io::Result<ServerConfig> {
    let certs = config.certs.clone();
    let server = &config.server;

    let cert = CertificateDer::pem_slice_iter(certs.tls_cert.expose(config).await.peek().as_ref())
        .map(|it| it.map_err(io::Error::other))
        .collect::<Result<Vec<_>, _>>()?;

    let priv_key =
        PrivateKeyDer::from_pem_slice(certs.tls_key.expose(config).await.peek().as_ref())
            .map_err(|_| io::Error::other("Could not parse pem file"))?;

    let mut roots = rustls::RootCertStore::empty();

    for ca in CertificateDer::pem_slice_iter(certs.root_ca.expose(config).await.peek().as_ref()) {
        roots
            .add(ca.map_err(io::Error::other)?)
            .map_err(io::Error::other)?;
    }

    let auth = WebPkiClientVerifier::builder(Arc::new(roots))
        .build()
        .map_err(io::Error::other)?;

    let mut config = ServerConfig::builder()
        .with_client_cert_verifier(auth)
        .with_single_cert(cert, priv_key)
        .map_err(io::Error::other)?;

    // ALPN decides whether concurrent requests from one client share a single
    // multiplexed connection (h2) or take one connection each (http/1.1).
    config.alpn_protocols = if server.enable_http2 {
        vec![b"h2".to_vec(), b"http/1.1".to_vec()]
    } else {
        vec![b"http/1.1".to_vec()]
    };

    Ok(config)
}
