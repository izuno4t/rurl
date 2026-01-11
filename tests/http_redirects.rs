use reqwest::header::{AUTHORIZATION, COOKIE};
use rurl::config::{Config, HttpMethod};
use rurl::error::RurlError;
use rurl::http::HttpClient;
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn can_bind_localhost() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

async fn received_requests(server: &MockServer) -> Vec<wiremock::Request> {
    server.received_requests().await.expect("requests")
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_follow_redirect_get() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/start"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/final"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/final"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let config = Config {
        url: format!("{}/start", server.uri()),
        method: HttpMethod::Get,
        follow_redirects: true,
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response_history = client
        .execute_with_history()
        .await
        .expect("request should succeed");
    assert_eq!(response_history.response.status(), 200);
    assert_eq!(response_history.chain.len(), 2);

    let requests = received_requests(&server).await;
    assert_eq!(requests.len(), 2);
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_post_redirect_switches_to_get_when_not_explicit() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/start"))
        .and(body_string("payload"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/final"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/final"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let config = Config {
        url: format!("{}/start", server.uri()),
        method: HttpMethod::Post,
        data: Some("payload".to_string()),
        follow_redirects: true,
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response_history = client
        .execute_with_history()
        .await
        .expect("request should succeed");
    assert_eq!(response_history.response.status(), 200);
    assert_eq!(response_history.chain.len(), 2);

    let requests = received_requests(&server).await;
    assert!(requests
        .iter()
        .any(|req| req.method.as_str() == "POST" && req.url.path() == "/start"));
    assert!(requests
        .iter()
        .any(|req| req.method.as_str() == "GET" && req.url.path() == "/final"));
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_post_redirect_keeps_method_when_explicit() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/start"))
        .and(body_string("payload"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/final"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/final"))
        .and(body_string("payload"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let config = Config {
        url: format!("{}/start", server.uri()),
        method: HttpMethod::Post,
        data: Some("payload".to_string()),
        request_method_explicit: true,
        follow_redirects: true,
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response_history = client
        .execute_with_history()
        .await
        .expect("request should succeed");
    assert_eq!(response_history.response.status(), 200);
    assert_eq!(response_history.chain.len(), 2);

    let requests = received_requests(&server).await;
    assert!(requests
        .iter()
        .any(|req| req.method.as_str() == "POST" && req.url.path() == "/final"));
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_redirect_limit_exceeded() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/start"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/next"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/next"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/final"))
        .mount(&server)
        .await;

    let config = Config {
        url: format!("{}/start", server.uri()),
        method: HttpMethod::Get,
        follow_redirects: true,
        max_redirects: Some(1),
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let result = client.execute_with_history().await;
    match result {
        Err(RurlError::RedirectLimitExceeded(1)) => {}
        Err(err) => panic!("unexpected error: {err}"),
        Ok(_) => panic!("expected redirect limit error"),
    }

    let requests = received_requests(&server).await;
    assert_eq!(requests.len(), 2);
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_sensitive_headers_not_forwarded_to_other_origin() {
    if !can_bind_localhost() {
        return;
    }

    let start_server = MockServer::start().await;
    let target_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/start"))
        .and(header(AUTHORIZATION.as_str(), "Bearer token"))
        .and(header(COOKIE.as_str(), "session=abc"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("Location", format!("{}/final", target_server.uri())),
        )
        .mount(&start_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/final"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&target_server)
        .await;

    let mut config = Config {
        url: format!("{}/start", start_server.uri()),
        method: HttpMethod::Get,
        follow_redirects: true,
        ..Config::default()
    };
    config
        .headers
        .insert("Authorization".to_string(), "Bearer token".to_string());
    config
        .headers
        .insert("Cookie".to_string(), "session=abc".to_string());

    let client = HttpClient::new(config).expect("client should build");
    let response_history = client
        .execute_with_history()
        .await
        .expect("request should succeed");
    assert_eq!(response_history.response.status(), 200);

    let requests = received_requests(&target_server).await;
    assert_eq!(requests.len(), 1);
    assert!(requests[0].headers.get(AUTHORIZATION.as_str()).is_none());
    assert!(requests[0].headers.get(COOKIE.as_str()).is_none());
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_location_trusted_forwards_sensitive_headers() {
    if !can_bind_localhost() {
        return;
    }

    let start_server = MockServer::start().await;
    let target_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/start"))
        .and(header(AUTHORIZATION.as_str(), "Bearer token"))
        .and(header(COOKIE.as_str(), "session=abc"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("Location", format!("{}/final", target_server.uri())),
        )
        .mount(&start_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/final"))
        .and(header(AUTHORIZATION.as_str(), "Bearer token"))
        .and(header(COOKIE.as_str(), "session=abc"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&target_server)
        .await;

    let mut config = Config {
        url: format!("{}/start", start_server.uri()),
        method: HttpMethod::Get,
        follow_redirects: true,
        location_trusted: true,
        ..Config::default()
    };
    config
        .headers
        .insert("Authorization".to_string(), "Bearer token".to_string());
    config
        .headers
        .insert("Cookie".to_string(), "session=abc".to_string());

    let client = HttpClient::new(config).expect("client should build");
    let response_history = client
        .execute_with_history()
        .await
        .expect("request should succeed");
    assert_eq!(response_history.response.status(), 200);

    let requests = received_requests(&target_server).await;
    assert_eq!(requests.len(), 1);
    assert!(requests[0].headers.get(AUTHORIZATION.as_str()).is_some());
    assert!(requests[0].headers.get(COOKIE.as_str()).is_some());
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_post301_keeps_post() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/start"))
        .and(body_string("payload"))
        .respond_with(ResponseTemplate::new(301).insert_header("Location", "/final"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/final"))
        .and(body_string("payload"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let config = Config {
        url: format!("{}/start", server.uri()),
        method: HttpMethod::Post,
        data: Some("payload".to_string()),
        follow_redirects: true,
        post301: true,
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response_history = client
        .execute_with_history()
        .await
        .expect("request should succeed");
    assert_eq!(response_history.response.status(), 200);

    let requests = received_requests(&server).await;
    assert_eq!(requests.len(), 2);
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_post302_keeps_post() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/start"))
        .and(body_string("payload"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/final"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/final"))
        .and(body_string("payload"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let config = Config {
        url: format!("{}/start", server.uri()),
        method: HttpMethod::Post,
        data: Some("payload".to_string()),
        follow_redirects: true,
        post302: true,
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response_history = client
        .execute_with_history()
        .await
        .expect("request should succeed");
    assert_eq!(response_history.response.status(), 200);

    let requests = received_requests(&server).await;
    assert_eq!(requests.len(), 2);
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_post303_keeps_post() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/start"))
        .and(body_string("payload"))
        .respond_with(ResponseTemplate::new(303).insert_header("Location", "/final"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/final"))
        .and(body_string("payload"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let config = Config {
        url: format!("{}/start", server.uri()),
        method: HttpMethod::Post,
        data: Some("payload".to_string()),
        follow_redirects: true,
        post303: true,
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response_history = client
        .execute_with_history()
        .await
        .expect("request should succeed");
    assert_eq!(response_history.response.status(), 200);

    let requests = received_requests(&server).await;
    assert_eq!(requests.len(), 2);
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_put_redirect_303_switches_to_get() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/start"))
        .and(body_string("payload"))
        .respond_with(ResponseTemplate::new(303).insert_header("Location", "/final"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/final"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let config = Config {
        url: format!("{}/start", server.uri()),
        method: HttpMethod::Put,
        data: Some("payload".to_string()),
        follow_redirects: true,
        ..Config::default()
    };

    let client = HttpClient::new(config).expect("client should build");
    let response_history = client
        .execute_with_history()
        .await
        .expect("request should succeed");
    assert_eq!(response_history.response.status(), 200);

    let requests = received_requests(&server).await;
    assert!(requests
        .iter()
        .any(|req| req.method.as_str() == "GET" && req.url.path() == "/final"));
}
