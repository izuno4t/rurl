use httpmock::Method::{GET, POST};
use httpmock::MockServer;
use reqwest::header::{AUTHORIZATION, COOKIE};
use rurl::config::{Config, HttpMethod};
use rurl::error::RurlError;
use rurl::http::HttpClient;

fn can_bind_localhost() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

#[tokio::test]
async fn test_follow_redirect_get() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start();
    let start = server.mock(|when, then| {
        when.method(GET).path("/start");
        then.status(302).header("Location", "/final");
    });
    let final_mock = server.mock(|when, then| {
        when.method(GET).path("/final");
        then.status(200).body("ok");
    });

    let config = Config {
        url: format!("{}/start", server.url("")),
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

    start.assert();
    final_mock.assert();
}

#[tokio::test]
async fn test_post_redirect_switches_to_get_when_not_explicit() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start();
    let start = server.mock(|when, then| {
        when.method(POST).path("/start").body("payload");
        then.status(302).header("Location", "/final");
    });
    let final_mock = server.mock(|when, then| {
        when.method(GET).path("/final");
        then.status(200).body("ok");
    });

    let config = Config {
        url: format!("{}/start", server.url("")),
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

    start.assert();
    final_mock.assert();
}

#[tokio::test]
async fn test_post_redirect_keeps_method_when_explicit() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start();
    let start = server.mock(|when, then| {
        when.method(POST).path("/start").body("payload");
        then.status(302).header("Location", "/final");
    });
    let final_mock = server.mock(|when, then| {
        when.method(POST).path("/final").body("payload");
        then.status(200).body("ok");
    });

    let config = Config {
        url: format!("{}/start", server.url("")),
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

    start.assert();
    final_mock.assert();
}

#[tokio::test]
async fn test_redirect_limit_exceeded() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start();
    let start = server.mock(|when, then| {
        when.method(GET).path("/start");
        then.status(302).header("Location", "/next");
    });
    let next = server.mock(|when, then| {
        when.method(GET).path("/next");
        then.status(302).header("Location", "/final");
    });

    let config = Config {
        url: format!("{}/start", server.url("")),
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

    start.assert();
    next.assert();
}

#[tokio::test]
async fn test_sensitive_headers_not_forwarded_to_other_origin() {
    if !can_bind_localhost() {
        return;
    }

    let start_server = MockServer::start();
    let target_server = MockServer::start();
    let start = start_server.mock(|when, then| {
        when.method(GET)
            .path("/start")
            .header(AUTHORIZATION.as_str(), "Bearer token")
            .header(COOKIE.as_str(), "session=abc");
        then.status(302)
            .header("Location", target_server.url("/final"));
    });
    let auth_present = target_server.mock(|when, then| {
        when.method(GET)
            .path("/final")
            .header(AUTHORIZATION.as_str(), "Bearer token");
        then.status(401).body("auth header should not be forwarded");
    });
    let cookie_present = target_server.mock(|when, then| {
        when.method(GET)
            .path("/final")
            .header(COOKIE.as_str(), "session=abc");
        then.status(401)
            .body("cookie header should not be forwarded");
    });
    let final_mock = target_server.mock(|when, then| {
        when.method(GET).path("/final");
        then.status(200).body("ok");
    });

    let mut config = Config {
        url: format!("{}/start", start_server.url("")),
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

    start.assert();
    final_mock.assert();
    auth_present.assert_hits(0);
    cookie_present.assert_hits(0);
}

#[tokio::test]
async fn test_location_trusted_forwards_sensitive_headers() {
    if !can_bind_localhost() {
        return;
    }

    let start_server = MockServer::start();
    let target_server = MockServer::start();
    let start = start_server.mock(|when, then| {
        when.method(GET)
            .path("/start")
            .header(AUTHORIZATION.as_str(), "Bearer token")
            .header(COOKIE.as_str(), "session=abc");
        then.status(302)
            .header("Location", target_server.url("/final"));
    });
    let final_mock = target_server.mock(|when, then| {
        when.method(GET)
            .path("/final")
            .header(AUTHORIZATION.as_str(), "Bearer token")
            .header(COOKIE.as_str(), "session=abc");
        then.status(200).body("ok");
    });

    let mut config = Config {
        url: format!("{}/start", start_server.url("")),
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

    start.assert();
    final_mock.assert();
}

#[tokio::test]
async fn test_post301_keeps_post() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start();
    let start = server.mock(|when, then| {
        when.method(POST).path("/start").body("payload");
        then.status(301).header("Location", "/final");
    });
    let final_mock = server.mock(|when, then| {
        when.method(POST).path("/final").body("payload");
        then.status(200).body("ok");
    });

    let config = Config {
        url: format!("{}/start", server.url("")),
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

    start.assert();
    final_mock.assert();
}

#[tokio::test]
async fn test_post302_keeps_post() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start();
    let start = server.mock(|when, then| {
        when.method(POST).path("/start").body("payload");
        then.status(302).header("Location", "/final");
    });
    let final_mock = server.mock(|when, then| {
        when.method(POST).path("/final").body("payload");
        then.status(200).body("ok");
    });

    let config = Config {
        url: format!("{}/start", server.url("")),
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

    start.assert();
    final_mock.assert();
}

#[tokio::test]
async fn test_post303_keeps_post() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start();
    let start = server.mock(|when, then| {
        when.method(POST).path("/start").body("payload");
        then.status(303).header("Location", "/final");
    });
    let final_mock = server.mock(|when, then| {
        when.method(POST).path("/final").body("payload");
        then.status(200).body("ok");
    });

    let config = Config {
        url: format!("{}/start", server.url("")),
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

    start.assert();
    final_mock.assert();
}

#[tokio::test]
async fn test_put_redirect_303_switches_to_get() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start();
    let start = server.mock(|when, then| {
        when.method(httpmock::Method::PUT)
            .path("/start")
            .body("payload");
        then.status(303).header("Location", "/final");
    });
    let final_mock = server.mock(|when, then| {
        when.method(GET).path("/final");
        then.status(200).body("ok");
    });

    let config = Config {
        url: format!("{}/start", server.url("")),
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

    start.assert();
    final_mock.assert();
}
