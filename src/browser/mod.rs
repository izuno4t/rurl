//! Browser cookie extraction module
//!
//! This module handles extracting cookies from various browsers
//! across different operating systems.

use crate::config::{Browser, BrowserCookieConfig};
use crate::error::Result;
use std::collections::HashMap;

pub mod chrome;
pub mod edge;
pub mod firefox;
pub mod safari;

/// Represents a browser cookie
#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub expires: Option<i64>,
}

/// Cookie store for managing extracted cookies
pub type CookieStore = HashMap<String, Vec<Cookie>>;

/// Main interface for extracting browser cookies
pub struct BrowserCookieExtractor {
    config: BrowserCookieConfig,
}

impl BrowserCookieExtractor {
    /// Create a new cookie extractor with the given configuration
    pub fn new(config: BrowserCookieConfig) -> Self {
        Self { config }
    }

    /// Extract cookies for the specified domain
    pub async fn extract_cookies(&self) -> Result<CookieStore> {
        match self.config.browser {
            Browser::Chrome => chrome::extract_cookies(&self.config).await,
            Browser::Firefox => firefox::extract_cookies(&self.config).await,
            Browser::Safari => safari::extract_cookies(&self.config).await,
            Browser::Edge => edge::extract_cookies(&self.config).await,
            Browser::Brave => {
                chrome::extract_chromium_cookies(chrome::ChromiumBrowser::Brave, &self.config)
            }
            Browser::Opera => {
                chrome::extract_chromium_cookies(chrome::ChromiumBrowser::Opera, &self.config)
            }
            Browser::Vivaldi => {
                chrome::extract_chromium_cookies(chrome::ChromiumBrowser::Vivaldi, &self.config)
            }
            Browser::Whale => {
                chrome::extract_chromium_cookies(chrome::ChromiumBrowser::Whale, &self.config)
            }
        }
    }

    /// Convert cookies to HTTP header format
    pub fn cookies_to_header(&self, cookies: &[Cookie]) -> String {
        cookies
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ")
    }
}
