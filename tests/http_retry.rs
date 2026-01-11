use rurl::config::{Config, HttpMethod};
use rurl::http::HttpClient;
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn can_bind_localhost() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_retries_on_http_500() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/retry"))
        .respond_with(ResponseTemplate::new(500).set_body_string("try again"))
        .mount(&server)
        .await;

    let config = Config {
        url: format!("{}/retry", server.uri()),
        method: HttpMethod::Get,
        retry_count: 1,
        retry_delay: Duration::from_millis(0),
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response_history = client
        .execute_with_history()
        .await
        .expect("request should succeed");
    assert_eq!(response_history.response.status(), 500);

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 2);
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_retries_on_retry_after_header() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/retry-after"))
        .respond_with(
            ResponseTemplate::new(503)
                .insert_header("Retry-After", "0")
                .set_body_string("retry"),
        )
        .mount(&server)
        .await;

    let config = Config {
        url: format!("{}/retry-after", server.uri()),
        method: HttpMethod::Get,
        retry_count: 1,
        retry_delay: Duration::from_millis(0),
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response_history = client
        .execute_with_history()
        .await
        .expect("request should succeed");
    assert_eq!(response_history.response.status(), 503);

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 2);
}
