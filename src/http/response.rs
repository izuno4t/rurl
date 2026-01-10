//! HTTP response handling and formatting

use crate::error::Result;
use serde_json::Value;

/// Response formatter for different output formats
pub struct ResponseFormatter {
    format_json: bool,
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