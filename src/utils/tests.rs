use super::{FileUtils, StringUtils, UrlUtils};
use crate::error::RurlError;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use url::Url;

#[test]
fn validate_url_adds_scheme() {
    let url = UrlUtils::validate_url("example.com").expect("valid url");
    assert_eq!(url.scheme(), "http");
    assert_eq!(url.host_str(), Some("example.com"));
}

#[test]
fn validate_url_rejects_invalid_input() {
    let err = UrlUtils::validate_url("http://").expect_err("invalid url");
    assert!(matches!(err, RurlError::InvalidUrl(_)));
}

#[test]
fn validate_url_accepts_https() {
    let url = UrlUtils::validate_url("https://example.com/path").expect("valid url");
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.path(), "/path");
}

#[test]
fn extract_domain_handles_ip_and_hostname() {
    let hostname = Url::parse("http://example.com/path").expect("valid url");
    assert_eq!(
        UrlUtils::extract_domain(&hostname),
        Some("example.com".to_string())
    );
    let ip = Url::parse("http://127.0.0.1/").expect("valid url");
    assert_eq!(UrlUtils::extract_domain(&ip), None);
}

#[test]
fn expand_path_expands_home() {
    let home = dirs::home_dir().expect("home dir");
    let path = FileUtils::expand_path("~/rurl-test").expect("expanded");
    assert_eq!(path, home.join("rurl-test"));
}

#[test]
fn expand_path_leaves_non_tilde_unchanged() {
    let path = FileUtils::expand_path("/tmp/rurl").expect("expanded");
    assert_eq!(path, PathBuf::from("/tmp/rurl"));
}

#[test]
fn check_file_readable_validates_paths() {
    let temp = tempdir().expect("tempdir");
    let file_path = temp.path().join("file.txt");
    fs::write(&file_path, "data").expect("write file");
    FileUtils::check_file_readable(&file_path).expect("readable file");

    let err =
        FileUtils::check_file_readable(&temp.path().join("missing")).expect_err("missing file");
    assert!(matches!(err, RurlError::FileNotFound(_)));

    let err = FileUtils::check_file_readable(temp.path()).expect_err("dir path");
    assert!(matches!(err, RurlError::Config(_)));
}

#[test]
fn parse_header_splits_key_value() {
    let (key, value) = StringUtils::parse_header("X-Test: value").expect("header");
    assert_eq!(key, "X-Test");
    assert_eq!(value, "value");

    let err = StringUtils::parse_header("missing").expect_err("invalid header");
    assert!(matches!(err, RurlError::Config(_)));
}

#[test]
fn parse_timeout_parses_suffixes() {
    assert_eq!(
        StringUtils::parse_timeout("10").expect("seconds"),
        std::time::Duration::from_secs(10)
    );
    assert_eq!(
        StringUtils::parse_timeout("2m").expect("minutes"),
        std::time::Duration::from_secs(120)
    );
    assert_eq!(
        StringUtils::parse_timeout("1h").expect("hours"),
        std::time::Duration::from_secs(3600)
    );

    let err = StringUtils::parse_timeout("5x").expect_err("invalid suffix");
    assert!(matches!(err, RurlError::Config(_)));

    let err = StringUtils::parse_timeout("xs").expect_err("invalid number");
    assert!(matches!(err, RurlError::Config(_)));
}
