use rurl::config::Config;
use rurl::http::HttpClient;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn can_bind_localhost() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_basic_auth_header_sent() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/auth"))
        .and(header("Authorization", "Basic dXNlcjpwYXNz"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let config = Config {
        url: format!("{}/auth", server.uri()),
        auth_username: Some("user".to_string()),
        auth_password: Some("pass".to_string()),
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response = client.execute().await.expect("request should succeed");
    assert_eq!(response.status(), 200);

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 1);
}
