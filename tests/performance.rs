use assert_cmd::cargo::cargo_bin_cmd;
use std::process::Command;
use std::time::{Duration, Instant};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn can_bind_localhost() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

fn curl_available() -> bool {
    Command::new("curl").arg("--version").output().is_ok()
}

fn run_rurl(url: &str, iterations: u32) -> Duration {
    let mut total = Duration::from_secs(0);
    for _ in 0..iterations {
        let start = Instant::now();
        let output = cargo_bin_cmd!("rurl")
            .arg(url)
            .arg("--no-progress-meter")
            .arg("-s")
            .output()
            .expect("run rurl");
        assert!(output.status.success());
        total += start.elapsed();
    }
    total
}

fn run_curl(url: &str, iterations: u32) -> Duration {
    let mut total = Duration::from_secs(0);
    for _ in 0..iterations {
        let start = Instant::now();
        let output = Command::new("curl")
            .arg("-s")
            .arg(url)
            .output()
            .expect("run curl");
        assert!(output.status.success());
        total += start.elapsed();
    }
    total
}

fn scaled_duration(duration: Duration, factor: u32) -> Duration {
    let nanos = duration.as_nanos() * u128::from(factor);
    Duration::from_nanos(nanos.min(u128::from(u64::MAX)) as u64)
}

#[cfg_attr(miri, ignore)]
#[tokio::test]
async fn test_local_performance_against_curl() {
    if !can_bind_localhost() || !curl_available() {
        return;
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/perf"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let url = format!("{}/perf", server.uri());
    let iterations = 5;
    let rurl_time = run_rurl(&url, iterations);
    let curl_time = run_curl(&url, iterations);

    if curl_time.as_nanos() > 0 {
        let max = scaled_duration(curl_time, 50);
        assert!(rurl_time <= max);
    }
}
