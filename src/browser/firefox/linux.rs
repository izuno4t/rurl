use crate::browser::CookieStore;
use crate::config::BrowserCookieConfig;
use crate::error::{Result, RurlError};

pub fn extract_cookies(_config: &BrowserCookieConfig) -> Result<CookieStore> {
    Err(RurlError::Unsupported(
        "Firefox cookie extraction on Linux is not implemented yet".to_string(),
    ))
}
