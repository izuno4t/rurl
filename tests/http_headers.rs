use httpmock::MockServer;
use rurl::config::Config;
use rurl::http::HttpClient;

fn can_bind_localhost() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

#[tokio::test]
async fn test_custom_header_sent() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path("/headers")
            .header("X-Test-Header", "rurl");
        then.status(200).body("ok");
    });

    let config = Config {
        url: format!("{}/headers", server.url("")),
        headers: [("X-Test-Header".to_string(), "rurl".to_string())]
            .into_iter()
            .collect(),
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response = client.execute().await.expect("request should succeed");
    assert_eq!(response.status(), 200);

    mock.assert();
}
