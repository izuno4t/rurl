//! Microsoft Edge browser cookie extraction

use crate::config::BrowserCookieConfig;
use crate::error::{Result, RurlError};
use crate::browser::{Cookie, CookieStore};

/// Extract cookies from Microsoft Edge browser
pub async fn extract_cookies(
    config: &BrowserCookieConfig,
    domain: Option<&str>,
) -> Result<CookieStore> {
    // Edge uses Chromium base, so we can reuse Chrome logic with different paths
    // This is a placeholder for the actual implementation
    Err(RurlError::Unsupported("Edge cookie extraction not yet implemented".to_string()))
}