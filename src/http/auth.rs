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

#[cfg(test)]
mod tests {
    use super::Auth;

    #[test]
    fn basic_auth_formats_header() {
        let header = Auth::basic_auth("user", "pass");
        assert!(header.starts_with("Basic "));
        assert!(header.contains("dXNlcjpwYXNz"));
    }

    #[test]
    fn bearer_token_formats_header() {
        let header = Auth::bearer_token("token");
        assert_eq!(header, "Bearer token");
    }

    #[test]
    fn parse_user_pass_splits_on_colon() {
        let (user, pass) = Auth::parse_user_pass("user:pass").expect("parsed");
        assert_eq!(user, "user");
        assert_eq!(pass, "pass");

        let (user, pass) = Auth::parse_user_pass("user").expect("parsed");
        assert_eq!(user, "user");
        assert!(pass.is_empty());

        let (user, pass) = Auth::parse_user_pass("user:pass:extra").expect("parsed");
        assert_eq!(user, "user");
        assert_eq!(pass, "pass:extra");
    }
}
