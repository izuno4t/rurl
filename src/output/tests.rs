use super::{
    decode_body_with_charset, extract_charset, format_response_headers, http_version_label,
    progress_line, OutputWriter, ProgressReporter,
};
use crate::config::OutputConfig;
use encoding_rs::WINDOWS_1252;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{StatusCode, Version};
use tempfile::tempdir;

#[test]
fn extract_charset_parses_case_insensitive() {
    assert_eq!(
        extract_charset(Some("text/plain; charset=utf-8")),
        Some("utf-8".to_string())
    );
    assert_eq!(
        extract_charset(Some("text/plain; CHARSET=iso-8859-1")),
        Some("iso-8859-1".to_string())
    );
    assert_eq!(extract_charset(Some("text/plain")), None);
}

#[test]
fn decode_body_with_charset_uses_declared_encoding() {
    let (encoded, _, _) = WINDOWS_1252.encode("\u{00A3}");
    let decoded = decode_body_with_charset(
        encoded.into_owned(),
        Some("text/plain; charset=windows-1252"),
    )
    .expect("decoded");
    assert_eq!(decoded, "\u{00A3}");
}

#[test]
fn decode_body_with_charset_falls_back_on_unknown_charset() {
    let body = vec![0xE3, 0x81, 0x82]; // "あ" in UTF-8
    let decoded =
        decode_body_with_charset(body, Some("text/plain; charset=unknown")).expect("decoded");
    assert!(decoded.contains('あ'));
}

#[test]
fn progress_line_formats_with_and_without_total() {
    assert_eq!(progress_line(5, None), "\r5 bytes");
    assert_eq!(progress_line(50, Some(100)), "\r50 / 100 bytes (50%)");
    assert_eq!(progress_line(150, Some(100)), "\r150 / 100 bytes (100%)");
}

#[test]
fn progress_reporter_respects_rate_limit_and_finish() {
    let mut reporter = ProgressReporter::new(true, Some(200));
    reporter.update(50);
    reporter.finish(200);
    assert!(reporter.rendered());
}

#[test]
fn progress_reporter_renders_on_finish_without_updates() {
    let mut reporter = ProgressReporter::new(true, Some(10));
    reporter.finish(10);
    assert!(reporter.rendered());
}

#[test]
fn http_version_label_maps_known_versions() {
    assert_eq!(http_version_label(Version::HTTP_11), "HTTP/1.1");
    assert_eq!(http_version_label(Version::HTTP_2), "HTTP/2");
}

#[test]
fn format_response_headers_includes_status_and_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("x-test", HeaderValue::from_static("value"));
    let output = format_response_headers(Version::HTTP_11, StatusCode::OK, &headers);
    assert!(output.starts_with("HTTP/1.1 200 OK\n"));
    assert!(output.contains("x-test: value\n"));
    assert!(output.ends_with('\n'));
}

#[test]
fn output_writer_writes_to_file() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("out.txt");
    let writer = OutputWriter::new(OutputConfig {
        file: Some(path.clone()),
        verbose: false,
        silent: false,
        show_progress: false,
        format_json: false,
        include_headers: false,
    });
    writer.write("data").expect("write");
    let written = std::fs::read_to_string(path).expect("read");
    assert_eq!(written, "data");
}
