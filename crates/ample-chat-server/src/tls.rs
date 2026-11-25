use crate::error::ServerError;
use std::path::Path;

#[cfg(feature = "tls")]
use tokio_rustls::TlsAcceptor;

#[cfg(feature = "tls")]
pub fn load_tls_acceptor(cert_path: &Path, key_path: &Path) -> Result<TlsAcceptor, ServerError> {
    use std::fs::File;
    use std::io::BufReader;
    use std::sync::Arc;
    use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};

    let cert_file = File::open(cert_path).map_err(|e| {
        ServerError::Tls(format!(
            "Failed to open certificate file {}: {e}",
            cert_path.display()
        ))
    })?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ServerError::Tls(format!("Failed to parse certificates: {e}")))?;

    if certs.is_empty() {
        return Err(ServerError::Tls(
            "No certificates found in cert file".to_string(),
        ));
    }

    let key_file = File::open(key_path).map_err(|e| {
        ServerError::Tls(format!(
            "Failed to open private key file {}: {e}",
            key_path.display()
        ))
    })?;
    let mut key_reader = BufReader::new(key_file);
    let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut key_reader)
        .map_err(|e| ServerError::Tls(format!("Failed to read private key: {e}")))?
        .ok_or_else(|| ServerError::Tls("No private key found in key file".to_string()))?;

    let server_config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| ServerError::Tls(format!("Invalid TLS configuration: {e}")))?;

    Ok(TlsAcceptor::from(Arc::new(server_config)))
}

#[cfg(not(feature = "tls"))]
pub fn load_tls_acceptor(_cert_path: &Path, _key_path: &Path) -> Result<(), ServerError> {
    Err(ServerError::Tls(
        "TLS feature is not enabled for ample-chat-server".to_string(),
    ))
}
