#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
use rurl::browser::BrowserCookieExtractor;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
use rurl::config::{Browser, BrowserCookieConfig};
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
use rurl::error::RurlError;

#[cfg(target_os = "macos")]
use rusqlite::Connection;
#[cfg(target_os = "macos")]
use std::path::Path;
#[cfg(target_os = "macos")]
use tempfile::tempdir;

#[cfg(target_os = "macos")]
fn create_chrome_cookie_db(path: &Path) {
    let conn = Connection::open(path).expect("open chrome db");
    conn.execute("CREATE TABLE meta (key TEXT, value TEXT)", [])
        .expect("create meta");
    conn.execute("INSERT INTO meta (key, value) VALUES ('version', '24')", [])
        .expect("insert meta");
    conn.execute(
        "CREATE TABLE cookies (
            host_key TEXT,
            name TEXT,
            value TEXT,
            encrypted_value BLOB,
            path TEXT,
            expires_utc INTEGER,
            is_secure INTEGER,
            is_httponly INTEGER
        )",
        [],
    )
    .expect("create cookies");
    conn.execute(
        "INSERT INTO cookies (
            host_key, name, value, encrypted_value, path, expires_utc, is_secure, is_httponly
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            "example.com",
            "session",
            "abc",
            Vec::<u8>::new(),
            "/",
            0i64,
            0i64,
            1i64,
        ),
    )
    .expect("insert cookie");
}

#[cfg(target_os = "macos")]
fn create_firefox_cookie_db(path: &Path) {
    let conn = Connection::open(path).expect("open firefox db");
    conn.execute("PRAGMA user_version = 16", [])
        .expect("set schema version");
    conn.execute(
        "CREATE TABLE moz_cookies (
            host TEXT,
            name TEXT,
            value TEXT,
            path TEXT,
            expiry INTEGER,
            isSecure INTEGER,
            isHttpOnly INTEGER
        )",
        [],
    )
    .expect("create moz_cookies");
    conn.execute(
        "INSERT INTO moz_cookies (
            host, name, value, path, expiry, isSecure, isHttpOnly
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        ("example.com", "session", "abc", "/", 1000i64, 0i64, 1i64),
    )
    .expect("insert cookie");
}

#[cfg(target_os = "macos")]
#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_extract_chrome_cookies_from_db() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("Cookies");
    create_chrome_cookie_db(&db_path);

    let config = BrowserCookieConfig {
        browser: Browser::Chrome,
        profile: Some(db_path.to_string_lossy().to_string()),
        container: None,
        keyring: None,
    };
    let extractor = BrowserCookieExtractor::new(config);
    let store = extractor.extract_cookies().await.expect("extract cookies");

    let cookies = store.values().next().expect("cookie list");
    assert!(cookies.iter().any(|cookie| cookie.name == "session"));
}

#[cfg(target_os = "macos")]
#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_extract_firefox_cookies_from_db() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("cookies.sqlite");
    create_firefox_cookie_db(&db_path);

    let config = BrowserCookieConfig {
        browser: Browser::Firefox,
        profile: Some(db_path.to_string_lossy().to_string()),
        container: None,
        keyring: None,
    };
    let extractor = BrowserCookieExtractor::new(config);
    let store = extractor.extract_cookies().await.expect("extract cookies");

    let cookies = store.values().next().expect("cookie list");
    assert!(cookies.iter().any(|cookie| cookie.name == "session"));
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
#[tokio::test]
async fn test_browser_cookie_extraction_unsupported() {
    let config = BrowserCookieConfig {
        browser: Browser::Chrome,
        profile: None,
        container: None,
        keyring: None,
    };
    let extractor = BrowserCookieExtractor::new(config);
    let err = extractor.extract_cookies().await.expect_err("unsupported");
    assert!(matches!(err, RurlError::Unsupported(_)));
}
