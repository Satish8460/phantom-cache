use axum_server::tls_rustls::RustlsConfig;
use std::path::Path;

#[derive(Clone)]
pub struct TlsServerConfig {
    pub rustls_config: RustlsConfig,
}

impl TlsServerConfig {
    pub async fn from_pem(cert_pem: &[u8], key_pem: &[u8]) -> Result<Self, std::io::Error> {
        let rustls_config = RustlsConfig::from_pem(cert_pem.to_vec(), key_pem.to_vec())
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(Self { rustls_config })
    }

    pub async fn from_pem_file<P: AsRef<Path>>(cert_path: P, key_path: P) -> Result<Self, std::io::Error> {
        let rustls_config = RustlsConfig::from_pem_file(cert_path, key_path)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(Self { rustls_config })
    }
}

