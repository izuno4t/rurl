use httpmock::Method::{DELETE, GET, POST, PUT};
use httpmock::MockServer;
use rurl::config::{Config, HttpMethod};
use rurl::http::HttpClient;

fn can_bind_localhost() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

async fn execute_with_method(server: &MockServer, method: HttpMethod) {
    let mock = server.mock(|when, then| {
        let when = match method {
            HttpMethod::Get => when.method(GET),
            HttpMethod::Post => when.method(POST),
            HttpMethod::Put => when.method(PUT),
            HttpMethod::Delete => when.method(DELETE),
            _ => panic!("unexpected method in test"),
        };
        when.path("/resource");
        then.status(200).body("ok");
    });

    let config = Config {
        url: format!("{}/resource", server.url("")),
        method,
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response = client.execute().await.expect("request should succeed");
    assert_eq!(response.status(), 200);

    mock.assert();
}

#[tokio::test]
async fn test_get_request() {
    if !can_bind_localhost() {
        return;
    }
    let server = MockServer::start();
    execute_with_method(&server, HttpMethod::Get).await;
}

#[tokio::test]
async fn test_post_request() {
    if !can_bind_localhost() {
        return;
    }
    let server = MockServer::start();
    execute_with_method(&server, HttpMethod::Post).await;
}

#[tokio::test]
async fn test_put_request() {
    if !can_bind_localhost() {
        return;
    }
    let server = MockServer::start();
    execute_with_method(&server, HttpMethod::Put).await;
}

#[tokio::test]
async fn test_delete_request() {
    if !can_bind_localhost() {
        return;
    }
    let server = MockServer::start();
    execute_with_method(&server, HttpMethod::Delete).await;
}
