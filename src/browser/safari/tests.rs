#[cfg(not(target_os = "macos"))]
#[tokio::test]
async fn safari_extraction_is_unsupported_off_macos() {
    use super::extract_cookies;
    use crate::config::{Browser, BrowserCookieConfig};
    use crate::error::RurlError;

    let cfg = BrowserCookieConfig {
        browser: Browser::Safari,
        profile: None,
        container: None,
        keyring: None,
    };
    let err = extract_cookies(&cfg).await.expect_err("unsupported");
    assert!(matches!(err, RurlError::Unsupported(_)));
}

#[cfg(target_os = "macos")]
mod macos_tests {
    use super::super::macos::test_support::{
        mac_absolute_to_unix, mac_epoch_offset, read_null_terminated_string_at, safari_cookie_path,
    };
    use std::fs;
    use tempfile::tempdir;

    use crate::config::{Browser, BrowserCookieConfig};
    use crate::error::RurlError;

    #[test]
    fn read_null_terminated_string_at_reads_value() {
        let data = b"test\0rest";
        let value = read_null_terminated_string_at(data, 0).expect("string");
        assert_eq!(value, "test");
    }

    #[test]
    fn read_null_terminated_string_at_rejects_missing_terminator() {
        let err = read_null_terminated_string_at(b"test", 0).expect_err("missing");
        let message = format!("{err}");
        assert!(message.contains("not terminated"));
    }

    #[test]
    fn read_null_terminated_string_at_rejects_out_of_bounds_offset() {
        let data = b"\0";
        let err = read_null_terminated_string_at(data, 1).expect_err("oob");
        let msg = format!("{err}");
        assert!(msg.contains("out of bounds"));
    }

    #[test]
    fn mac_absolute_to_unix_converts_seconds() {
        assert_eq!(mac_absolute_to_unix(0.0), mac_epoch_offset());
    }

    #[test]
    fn safari_cookie_path_accepts_custom_existing_file() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("Cookies.binarycookies");
        fs::write(&path, b"dummy").expect("write");
        let resolved = safari_cookie_path(Some(path.to_string_lossy().as_ref())).expect("resolve");
        assert_eq!(resolved, path);
    }

    #[test]
    fn safari_cookie_path_rejects_custom_missing_file() {
        let cfg = BrowserCookieConfig {
            browser: Browser::Safari,
            profile: Some("/nonexistent/Cookies.binarycookies".to_string()),
            container: None,
            keyring: None,
        };
        let err = safari_cookie_path(cfg.profile.as_deref()).expect_err("missing");
        assert!(matches!(err, RurlError::FileNotFound(_)));
    }
}
