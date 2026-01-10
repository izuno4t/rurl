use httpmock::Method::POST;
use httpmock::MockServer;
use rurl::config::{Config, HttpMethod};
use rurl::http::HttpClient;

fn can_bind_localhost() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

#[tokio::test]
async fn test_post_body_sent() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/body").body("payload");
        then.status(200).body("ok");
    });

    let config = Config {
        url: format!("{}/body", server.url("")),
        method: HttpMethod::Post,
        data: Some("payload".to_string()),
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response = client.execute().await.expect("request should succeed");
    assert_eq!(response.status(), 200);

    mock.assert();
}
