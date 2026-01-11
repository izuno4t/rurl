use rurl::config::{Config, HttpMethod};
use rurl::http::HttpClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn can_bind_localhost() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

async fn execute_with_method(server: &MockServer, http_method: HttpMethod) {
    let method_name = match http_method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Delete => "DELETE",
        _ => panic!("unexpected method in test"),
    };
    Mock::given(method(method_name))
        .and(path("/resource"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(server)
        .await;

    let config = Config {
        url: format!("{}/resource", server.uri()),
        method: http_method,
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
async fn test_get_request() {
    if !can_bind_localhost() {
        return;
    }
    let server = MockServer::start().await;
    execute_with_method(&server, HttpMethod::Get).await;
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_post_request() {
    if !can_bind_localhost() {
        return;
    }
    let server = MockServer::start().await;
    execute_with_method(&server, HttpMethod::Post).await;
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_put_request() {
    if !can_bind_localhost() {
        return;
    }
    let server = MockServer::start().await;
    execute_with_method(&server, HttpMethod::Put).await;
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_delete_request() {
    if !can_bind_localhost() {
        return;
    }
    let server = MockServer::start().await;
    execute_with_method(&server, HttpMethod::Delete).await;
}
