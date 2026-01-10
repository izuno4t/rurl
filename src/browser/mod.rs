//! Browser cookie extraction module
//!
//! This module handles extracting cookies from various browsers
//! across different operating systems.

use crate::config::{Browser, BrowserCookieConfig};
use crate::error::{Result, RurlError};
use std::collections::HashMap;

pub mod chrome;
pub mod firefox;
pub mod safari;
pub mod edge;

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
    pub async fn extract_cookies(&self, domain: Option<&str>) -> Result<CookieStore> {
        match self.config.browser {
            Browser::Chrome => chrome::extract_cookies(&self.config, domain).await,
            Browser::Firefox => firefox::extract_cookies(&self.config, domain).await,
            Browser::Safari => safari::extract_cookies(&self.config, domain).await,
            Browser::Edge => edge::extract_cookies(&self.config, domain).await,
            Browser::Brave => chrome::extract_cookies(&self.config, domain).await, // Brave uses Chrome base
            Browser::Opera => chrome::extract_cookies(&self.config, domain).await, // Opera uses Chromium
            Browser::Vivaldi => chrome::extract_cookies(&self.config, domain).await, // Vivaldi uses Chromium
            Browser::Whale => chrome::extract_cookies(&self.config, domain).await, // Whale uses Chromium
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