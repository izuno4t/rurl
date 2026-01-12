//! Microsoft Edge browser cookie extraction

use crate::browser::CookieStore;
use crate::config::BrowserCookieConfig;
use crate::error::Result;

/// Extract cookies from Microsoft Edge browser
pub async fn extract_cookies(config: &BrowserCookieConfig) -> Result<CookieStore> {
    crate::browser::chrome::extract_chromium_cookies(
        crate::browser::chrome::ChromiumBrowser::Edge,
        config,
    )
}

#[cfg(test)]
mod tests;
