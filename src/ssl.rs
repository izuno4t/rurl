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
