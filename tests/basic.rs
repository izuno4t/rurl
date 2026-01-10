use rurl::utils::UrlUtils;

#[test]
fn test_version() {
    assert!(!rurl::VERSION.is_empty());
}

#[test]
fn test_url_utils_adds_scheme() {
    let url = UrlUtils::validate_url("example.com").expect("URL should parse");
    assert_eq!(url.scheme(), "http");
}
