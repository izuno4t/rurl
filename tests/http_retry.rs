use httpmock::Method::GET;
use httpmock::MockServer;
use rurl::config::{Config, HttpMethod};
use rurl::http::HttpClient;
use std::time::Duration;

fn can_bind_localhost() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

#[tokio::test]
async fn test_retries_on_http_500() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/retry");
        then.status(500).body("try again");
    });

    let config = Config {
        url: format!("{}/retry", server.url("")),
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

    mock.assert_hits(2);
}
