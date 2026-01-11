//! HTTP response handling and formatting

use crate::error::Result;
use reqwest::header::HeaderMap;
use reqwest::{StatusCode, Version};
use serde_json::Value;

/// Response formatter for different output formats
pub struct ResponseFormatter {
    format_json: bool,
}

#[derive(Debug, Clone)]
pub struct ResponseInfo {
    pub version: Version,
    pub status: StatusCode,
    pub headers: HeaderMap,
}

pub struct ResponseHistory {
    pub response: reqwest::Response,
    pub chain: Vec<ResponseInfo>,
}

impl ResponseFormatter {
    pub fn new(format_json: bool) -> Self {
        Self { format_json }
    }

    /// Format response body based on content type
    pub fn format(&self, body: &str, content_type: Option<&str>) -> Result<String> {
        if self.format_json && self.is_json_content(content_type) {
            self.format_json_body(body)
        } else {
            Ok(body.to_string())
        }
    }

    fn is_json_content(&self, content_type: Option<&str>) -> bool {
        content_type
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false)
    }

    fn format_json_body(&self, body: &str) -> Result<String> {
        let value: Value = serde_json::from_str(body)?;
        Ok(serde_json::to_string_pretty(&value)?)
    }
}

#[cfg(test)]
mod tests {
    use super::ResponseFormatter;

    #[test]
    fn format_json_body_pretty_prints() {
        let formatter = ResponseFormatter::new(true);
        let body = r#"{"name":"rurl","version":1}"#;
        let formatted = formatter
            .format(body, Some("application/json"))
            .expect("format should succeed");
        assert_eq!(formatted, "{\n  \"name\": \"rurl\",\n  \"version\": 1\n}");
    }

    #[test]
    fn format_json_body_skips_when_disabled() {
        let formatter = ResponseFormatter::new(false);
        let body = r#"{"name":"rurl"}"#;
        let formatted = formatter
            .format(body, Some("application/json"))
            .expect("format should succeed");
        assert_eq!(formatted, body);
    }
}
