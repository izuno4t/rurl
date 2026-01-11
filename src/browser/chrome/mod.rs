//! Chrome/Chromium browser cookie extraction

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

/// Supported Chromium-based browsers on macOS, Linux, and Windows.
#[derive(Debug, Clone, Copy)]
pub enum ChromiumBrowser {
    Chrome,
    Edge,
    Brave,
    Opera,
    Vivaldi,
    Whale,
}

/// Extract cookies from Chrome browser
pub async fn extract_cookies(config: &BrowserCookieConfig) -> Result<CookieStore> {
    extract_chromium_cookies(ChromiumBrowser::Chrome, config)
}

pub fn extract_chromium_cookies(
    browser: ChromiumBrowser,
    config: &BrowserCookieConfig,
) -> Result<CookieStore> {
    #[cfg(target_os = "macos")]
    {
        macos::extract_chromium_cookies(browser, config)
    }
    #[cfg(target_os = "linux")]
    {
        linux::extract_chromium_cookies(browser, config)
    }
    #[cfg(target_os = "windows")]
    {
        windows::extract_chromium_cookies(browser, config)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = (browser, config);
        Err(RurlError::Unsupported(
            "Chromium cookie extraction is only implemented for macOS, Linux, and Windows"
                .to_string(),
        ))
    }
}
