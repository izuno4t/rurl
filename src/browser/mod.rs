//! Browser cookie extraction module
//!
//! This module handles extracting cookies from various browsers
//! across different operating systems.

use crate::config::{Browser, BrowserCookieConfig};
use crate::error::Result;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

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

    /// Filter cookies for a specific URL using standard matching rules
    pub fn cookies_for_url(&self, store: &CookieStore, url: &Url) -> Vec<Cookie> {
        let host = match url.host_str() {
            Some(host) => host.to_lowercase(),
            None => return Vec::new(),
        };
        let path = url.path();
        let is_https = url.scheme() == "https";
        let now = unix_timestamp_seconds();

        let mut matched = Vec::new();
        for cookies in store.values() {
            for cookie in cookies {
                if cookie.secure && !is_https {
                    continue;
                }
                if is_expired(cookie.expires, now) {
                    continue;
                }
                if !domain_matches(&host, &cookie.domain) {
                    continue;
                }
                if !path_matches(path, &cookie.path) {
                    continue;
                }
                matched.push(cookie.clone());
            }
        }
        matched
    }
}

fn unix_timestamp_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn is_expired(expires: Option<i64>, now: i64) -> bool {
    let expires = match expires {
        Some(expires) => expires,
        None => return false,
    };
    if expires > 100_000_000_000 {
        return false;
    }
    expires <= now
}

fn domain_matches(host: &str, cookie_domain: &str) -> bool {
    let cookie_domain = cookie_domain.trim().to_lowercase();
    if cookie_domain.is_empty() {
        return false;
    }
    if cookie_domain.starts_with('.') {
        let domain = cookie_domain.trim_start_matches('.');
        if domain.is_empty() {
            return false;
        }
        host == domain || host.ends_with(&format!(".{}", domain))
    } else {
        host == cookie_domain
    }
}

fn path_matches(request_path: &str, cookie_path: &str) -> bool {
    let cookie_path = if cookie_path.is_empty() {
        "/"
    } else {
        cookie_path
    };
    if request_path == cookie_path {
        return true;
    }
    if !request_path.starts_with(cookie_path) {
        return false;
    }
    cookie_path.ends_with('/') || request_path[cookie_path.len()..].starts_with('/')
}
