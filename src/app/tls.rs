use crate::config::Config;
use masking::PeekInterface;

use rustls::{pki_types::CertificateDer, server::WebPkiClientVerifier, ServerConfig};
use std::io;
use std::sync::Arc;

pub async fn from_config(config: &Config) -> io::Result<ServerConfig> {
    let certs = config.certs.clone();

    let cert = rustls_pemfile::certs(&mut certs.tls_cert.expose(config).await.peek().as_ref())
        .map(|it| it.map(|it| it.to_vec()))
        .collect::<Result<Vec<_>, _>>()?;

    let priv_key =
        rustls_pemfile::private_key(&mut certs.tls_key.expose(config).await.peek().as_ref())?
            .ok_or(io::Error::other("Could not parse pem file"))?;

    let cert = cert.into_iter().map(CertificateDer::from).collect();

    let mut roots = rustls::RootCertStore::empty();

    for ca in rustls_pemfile::certs(&mut certs.root_ca.expose(config).await.peek().as_ref()) {
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

    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(config)
}
