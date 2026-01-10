use httpmock::Method::GET;
use httpmock::MockServer;
use rurl::config::Config;
use rurl::http::HttpClient;

fn can_bind_localhost() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

#[tokio::test]
async fn test_basic_auth_header_sent() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/auth")
            .header("Authorization", "Basic dXNlcjpwYXNz");
        then.status(200).body("ok");
    });

    let config = Config {
        url: format!("{}/auth", server.url("")),
        auth_username: Some("user".to_string()),
        auth_password: Some("pass".to_string()),
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response = client.execute().await.expect("request should succeed");
    assert_eq!(response.status(), 200);

    mock.assert();
}
