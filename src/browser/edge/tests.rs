use super::extract_cookies;
use crate::config::{Browser, BrowserCookieConfig};

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn extract_cookies_returns_error_for_missing_profile() {
    let config = BrowserCookieConfig {
        browser: Browser::Edge,
        profile: Some("non-existent-profile".to_string()),
        container: None,
        keyring: None,
    };
    let result = extract_cookies(&config).await;
    assert!(result.is_err());
}
