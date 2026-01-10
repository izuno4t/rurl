//! HTTP authentication utilities

use crate::error::{Result, RurlError};
use base64::Engine;

/// Authentication types supported
#[derive(Debug, Clone)]
pub enum AuthType {
    Basic,
    Bearer,
    Digest,
}

/// Authentication helper
pub struct Auth;

impl Auth {
    /// Create basic auth header value
    pub fn basic_auth(username: &str, password: &str) -> String {
        let credentials = format!("{}:{}", username, password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        format!("Basic {}", encoded)
    }

    /// Create bearer token header value
    pub fn bearer_token(token: &str) -> String {
        format!("Bearer {}", token)
    }

    /// Parse user:password format
    pub fn parse_user_pass(input: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = input.splitn(2, ':').collect();
        match parts.as_slice() {
            [user, pass] => Ok((user.to_string(), pass.to_string())),
            [user] => Ok((user.to_string(), String::new())),
            _ => Err(RurlError::Auth("Invalid user:password format".to_string())),
        }
    }
}
