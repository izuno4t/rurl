use rurl::config::Config;
use rurl::http::HttpClient;
use rurl::VERSION;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn can_bind_localhost() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_custom_header_sent() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/headers"))
        .and(header("X-Test-Header", "rurl"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let config = Config {
        url: format!("{}/headers", server.uri()),
        headers: [("X-Test-Header".to_string(), "rurl".to_string())]
            .into_iter()
            .collect(),
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response = client.execute().await.expect("request should succeed");
    assert_eq!(response.status(), 200);

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 1);
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_user_agent_default_sent() {
    if !can_bind_localhost() {
        return;
    }

    let expected = format!("rurl/{}", VERSION);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ua"))
        .and(header("user-agent", expected.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let config = Config {
        url: format!("{}/ua", server.uri()),
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response = client.execute().await.expect("request should succeed");
    assert_eq!(response.status(), 200);
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_verbose_request_headers_path() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/verbose"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let mut config = Config {
        url: format!("{}/verbose", server.uri()),
        ..Config::default()
    };
    config.output.verbose = true;

    let client = HttpClient::new(config).expect("client should build");
    let response = client.execute().await.expect("request should succeed");
    assert_eq!(response.status(), 200);
}
