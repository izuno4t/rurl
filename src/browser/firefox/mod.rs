//! Firefox browser cookie extraction

use crate::browser::CookieStore;
use crate::config::BrowserCookieConfig;
use crate::error::Result;
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
use crate::error::RurlError;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// Extract cookies from Firefox browser
pub async fn extract_cookies(config: &BrowserCookieConfig) -> Result<CookieStore> {
    #[cfg(target_os = "macos")]
    {
        macos::extract_cookies(config)
    }
    #[cfg(target_os = "linux")]
    {
        linux::extract_cookies(config)
    }
    #[cfg(target_os = "windows")]
    {
        windows::extract_cookies(config)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = config;
        Err(RurlError::Unsupported(
            "Firefox cookie extraction is only implemented for macOS, Linux, and Windows"
                .to_string(),
        ))
    }
}
