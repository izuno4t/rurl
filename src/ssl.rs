//! SSL/TLS configuration and utilities

use crate::config::SslConfig;
use crate::error::{Result, RurlError};
use std::path::Path;

/// SSL/TLS certificate utilities
pub struct SslUtils;

impl SslUtils {
    /// Validate SSL configuration
    pub fn validate_config(config: &SslConfig) -> Result<()> {
        if let Some(ca_cert) = &config.ca_cert_file {
            if !ca_cert.exists() {
                return Err(RurlError::FileNotFound(format!(
                    "CA certificate file not found: {:?}",
                    ca_cert
                )));
            }
        }

        if let Some(cert) = &config.client_cert_file {
            if !cert.exists() {
                return Err(RurlError::FileNotFound(format!(
                    "Client certificate file not found: {:?}",
                    cert
                )));
            }
        }

        if let Some(key) = &config.client_key_file {
            if !key.exists() {
                return Err(RurlError::FileNotFound(format!(
                    "Client key file not found: {:?}",
                    key
                )));
            }
        }

        Ok(())
    }

    /// Read certificate file contents
    pub fn read_cert_file(path: &Path) -> Result<Vec<u8>> {
        std::fs::read(path).map_err(RurlError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::SslUtils;
    use crate::config::SslConfig;
    use crate::error::RurlError;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn validate_config_accepts_existing_files() {
        let temp = tempdir().expect("tempdir");
        let ca = temp.path().join("ca.pem");
        let cert = temp.path().join("client.pem");
        let key = temp.path().join("client.key");
        fs::write(&ca, "ca").expect("write ca");
        fs::write(&cert, "cert").expect("write cert");
        fs::write(&key, "key").expect("write key");
        let config = SslConfig {
            verify_certs: true,
            ca_cert_file: Some(ca),
            client_cert_file: Some(cert),
            client_key_file: Some(key),
        };
        SslUtils::validate_config(&config).expect("valid config");
    }

    #[test]
    fn validate_config_rejects_missing_files() {
        let temp = tempdir().expect("tempdir");
        let config = SslConfig {
            verify_certs: true,
            ca_cert_file: Some(temp.path().join("missing.pem")),
            client_cert_file: None,
            client_key_file: None,
        };
        let err = SslUtils::validate_config(&config).expect_err("missing");
        assert!(matches!(err, RurlError::FileNotFound(_)));
    }

    #[test]
    fn read_cert_file_reads_bytes() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("cert.pem");
        fs::write(&path, "data").expect("write");
        let data = SslUtils::read_cert_file(&path).expect("read");
        assert_eq!(data, b"data");
    }
}
