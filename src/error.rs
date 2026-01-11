//! Error handling for rurl

use thiserror::Error;

/// Main error type for rurl operations
#[derive(Error, Debug)]
pub enum RurlError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Browser cookie error: {0}")]
    BrowserCookie(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("SSL/TLS error: {0}")]
    Ssl(String),

    #[error("Proxy error: {0}")]
    Proxy(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network timeout")]
    Timeout,

    #[error("Redirect limit exceeded: {0}")]
    RedirectLimitExceeded(usize),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Unsupported operation: {0}")]
    Unsupported(String),
}

/// Result type alias for rurl operations
pub type Result<T> = std::result::Result<T, RurlError>;
