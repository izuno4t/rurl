//! Utility functions and helpers

use crate::error::{Result, RurlError};
use std::path::{Path, PathBuf};
use url::Url;

/// URL validation and parsing utilities
pub struct UrlUtils;

impl UrlUtils {
    /// Validate and normalize URL
    pub fn validate_url(input: &str) -> Result<Url> {
        // Add http:// if no scheme is provided
        let url_str = if input.contains("://") {
            input.to_string()
        } else {
            format!("http://{}", input)
        };

        Url::parse(&url_str)
            .map_err(|e| RurlError::InvalidUrl(format!("Invalid URL '{}': {}", input, e)))
    }

    /// Extract domain from URL for cookie filtering
    pub fn extract_domain(url: &Url) -> Option<String> {
        url.domain().map(|d| d.to_string())
    }
}

/// File system utilities
pub struct FileUtils;

impl FileUtils {
    /// Expand tilde (~) in file paths
    pub fn expand_path(path: &str) -> Result<PathBuf> {
        if path.starts_with('~') {
            if let Some(home_dir) = dirs::home_dir() {
                Ok(home_dir.join(&path[2..]))
            } else {
                Err(RurlError::Config(
                    "Cannot determine home directory".to_string(),
                ))
            }
        } else {
            Ok(PathBuf::from(path))
        }
    }

    /// Check if file exists and is readable
    pub fn check_file_readable(path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(RurlError::FileNotFound(format!(
                "File not found: {:?}",
                path
            )));
        }

        if !path.is_file() {
            return Err(RurlError::Config(format!("Path is not a file: {:?}", path)));
        }

        // Check if readable (basic check)
        std::fs::File::open(path).map_err(|e| {
            RurlError::PermissionDenied(format!("Cannot read file {:?}: {}", path, e))
        })?;

        Ok(())
    }
}

/// String utilities
pub struct StringUtils;

impl StringUtils {
    /// Parse key=value pairs from headers
    pub fn parse_header(input: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = input.splitn(2, ':').collect();
        match parts.as_slice() {
            [key, value] => {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                Ok((key, value))
            }
            _ => Err(RurlError::Config(format!(
                "Invalid header format: '{}'. Expected 'key: value'",
                input
            ))),
        }
    }

    /// Parse timeout values (supports suffixes like 's', 'm', 'h')
    pub fn parse_timeout(input: &str) -> Result<std::time::Duration> {
        if let Ok(seconds) = input.parse::<u64>() {
            return Ok(std::time::Duration::from_secs(seconds));
        }

        let (number_part, suffix) = if let Some(stripped) = input.strip_suffix('s') {
            (stripped, 1)
        } else if let Some(stripped) = input.strip_suffix('m') {
            (stripped, 60)
        } else if let Some(stripped) = input.strip_suffix('h') {
            (stripped, 3600)
        } else {
            return Err(RurlError::Config(format!(
                "Invalid timeout format: '{}'. Use number with optional suffix (s/m/h)",
                input
            )));
        };

        let number: u64 = number_part
            .parse()
            .map_err(|_| RurlError::Config(format!("Invalid timeout number: '{}'", number_part)))?;

        Ok(std::time::Duration::from_secs(number * suffix))
    }
}

#[cfg(test)]
mod tests;
