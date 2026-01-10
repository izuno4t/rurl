//! HTTP request handling

use crate::config::Config;
use crate::error::Result;

/// Request builder utilities
pub struct RequestBuilder {
    config: Config,
}

impl RequestBuilder {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
    
    /// Prepare request with browser cookies
    pub async fn with_browser_cookies(&mut self) -> Result<&mut Self> {
        // Implementation will integrate with browser cookie extraction
        Ok(self)
    }
}