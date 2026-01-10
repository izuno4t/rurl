//! Microsoft Edge browser cookie extraction

use crate::browser::CookieStore;
use crate::error::{Result, RurlError};

/// Extract cookies from Microsoft Edge browser
pub async fn extract_cookies() -> Result<CookieStore> {
    // Edge uses Chromium base, so we can reuse Chrome logic with different paths
    // This is a placeholder for the actual implementation
    Err(RurlError::Unsupported(
        "Edge cookie extraction not yet implemented".to_string(),
    ))
}
