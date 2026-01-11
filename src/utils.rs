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
mod tests {
    use super::{FileUtils, StringUtils, UrlUtils};
    use crate::error::RurlError;
    use std::fs;
    use tempfile::tempdir;
    use url::Url;

    #[test]
    fn validate_url_adds_scheme() {
        let url = UrlUtils::validate_url("example.com").expect("valid url");
        assert_eq!(url.scheme(), "http");
        assert_eq!(url.host_str(), Some("example.com"));
    }

    #[test]
    fn validate_url_rejects_invalid_input() {
        let err = UrlUtils::validate_url("http://").expect_err("invalid url");
        assert!(matches!(err, RurlError::InvalidUrl(_)));
    }

    #[test]
    fn extract_domain_handles_ip_and_hostname() {
        let hostname = Url::parse("http://example.com/path").expect("valid url");
        assert_eq!(
            UrlUtils::extract_domain(&hostname),
            Some("example.com".to_string())
        );
        let ip = Url::parse("http://127.0.0.1/").expect("valid url");
        assert_eq!(UrlUtils::extract_domain(&ip), None);
    }

    #[test]
    fn expand_path_expands_home() {
        let home = dirs::home_dir().expect("home dir");
        let path = FileUtils::expand_path("~/rurl-test").expect("expanded");
        assert_eq!(path, home.join("rurl-test"));
    }

    #[test]
    fn check_file_readable_validates_paths() {
        let temp = tempdir().expect("tempdir");
        let file_path = temp.path().join("file.txt");
        fs::write(&file_path, "data").expect("write file");
        FileUtils::check_file_readable(&file_path).expect("readable file");

        let err =
            FileUtils::check_file_readable(&temp.path().join("missing")).expect_err("missing file");
        assert!(matches!(err, RurlError::FileNotFound(_)));

        let err = FileUtils::check_file_readable(temp.path()).expect_err("dir path");
        assert!(matches!(err, RurlError::Config(_)));
    }

    #[test]
    fn parse_header_splits_key_value() {
        let (key, value) = StringUtils::parse_header("X-Test: value").expect("header");
        assert_eq!(key, "X-Test");
        assert_eq!(value, "value");

        let err = StringUtils::parse_header("missing").expect_err("invalid header");
        assert!(matches!(err, RurlError::Config(_)));
    }

    #[test]
    fn parse_timeout_parses_suffixes() {
        assert_eq!(
            StringUtils::parse_timeout("10").expect("seconds"),
            std::time::Duration::from_secs(10)
        );
        assert_eq!(
            StringUtils::parse_timeout("2m").expect("minutes"),
            std::time::Duration::from_secs(120)
        );
        assert_eq!(
            StringUtils::parse_timeout("1h").expect("hours"),
            std::time::Duration::from_secs(3600)
        );

        let err = StringUtils::parse_timeout("5x").expect_err("invalid suffix");
        assert!(matches!(err, RurlError::Config(_)));

        let err = StringUtils::parse_timeout("xs").expect_err("invalid number");
        assert!(matches!(err, RurlError::Config(_)));
    }
}
