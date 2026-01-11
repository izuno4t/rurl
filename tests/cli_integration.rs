use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_cli_help_succeeds() {
    let output = cargo_bin_cmd!("rurl")
        .arg("--help")
        .output()
        .expect("run rurl");
    assert!(output.status.success(), "help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage"), "help should include usage text");
}

fn can_bind_localhost() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_cli_outputs_body() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/body"))
        .respond_with(ResponseTemplate::new(200).set_body_string("hello"))
        .mount(&server)
        .await;

    let url = format!("{}/body", server.uri());
    let output = cargo_bin_cmd!("rurl")
        .arg(&url)
        .arg("--no-progress-meter")
        .output()
        .expect("run rurl");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello"));
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_cli_include_headers() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/headers"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-test", "value")
                .set_body_string("ok"),
        )
        .mount(&server)
        .await;

    let url = format!("{}/headers", server.uri());
    let output = cargo_bin_cmd!("rurl")
        .arg(&url)
        .arg("-i")
        .arg("--no-progress-meter")
        .output()
        .expect("run rurl");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("HTTP/1.1 200 OK"));
    assert!(stdout.contains("x-test: value"));
    assert!(stdout.contains("ok"));
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_cli_writes_output_file() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/file"))
        .respond_with(ResponseTemplate::new(200).set_body_string("file-body"))
        .mount(&server)
        .await;

    let dir = tempdir().expect("tempdir");
    let output_path = dir.path().join("out.txt");
    let url = format!("{}/file", server.uri());
    let output = cargo_bin_cmd!("rurl")
        .arg(&url)
        .arg("-o")
        .arg(&output_path)
        .arg("--no-progress-meter")
        .output()
        .expect("run rurl");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.is_empty());
    let written = std::fs::read_to_string(output_path).expect("read output file");
    assert_eq!(written, "file-body");
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_cli_sets_user_agent() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ua"))
        .and(header("user-agent", "rurl-test-agent"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let url = format!("{}/ua", server.uri());
    let output = cargo_bin_cmd!("rurl")
        .arg(&url)
        .arg("-A")
        .arg("rurl-test-agent")
        .arg("--no-progress-meter")
        .output()
        .expect("run rurl");

    assert!(output.status.success());
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 1);
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_cli_basic_auth_header() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/auth"))
        .and(header("authorization", "Basic dXNlcjpwYXNz"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let url = format!("{}/auth", server.uri());
    let output = cargo_bin_cmd!("rurl")
        .arg(&url)
        .arg("-u")
        .arg("user:pass")
        .arg("--no-progress-meter")
        .output()
        .expect("run rurl");

    assert!(output.status.success());
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 1);
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_cli_explicit_method() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/method"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let url = format!("{}/method", server.uri());
    let output = cargo_bin_cmd!("rurl")
        .arg(&url)
        .arg("-X")
        .arg("PUT")
        .arg("--no-progress-meter")
        .output()
        .expect("run rurl");

    assert!(output.status.success());
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 1);
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_cli_max_redirs_zero_returns_error() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/redir"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/next"))
        .mount(&server)
        .await;

    let url = format!("{}/redir", server.uri());
    let output = cargo_bin_cmd!("rurl")
        .arg(&url)
        .arg("-L")
        .arg("--max-redirs=0")
        .arg("--no-progress-meter")
        .output()
        .expect("run rurl");

    assert_eq!(output.status.code(), Some(47));
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 1);
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_cli_retries_on_http_error() {
    if !can_bind_localhost() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/retry"))
        .respond_with(ResponseTemplate::new(503).set_body_string("retry"))
        .mount(&server)
        .await;

    let url = format!("{}/retry", server.uri());
    let output = cargo_bin_cmd!("rurl")
        .arg(&url)
        .arg("--retry")
        .arg("1")
        .arg("--retry-delay")
        .arg("0")
        .arg("--no-progress-meter")
        .output()
        .expect("run rurl");

    assert!(output.status.success());
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 2);
}
