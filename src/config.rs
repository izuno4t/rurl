//! Configuration management for rurl

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use crate::error::Result;

/// Browser types supported for cookie extraction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Browser {
    Chrome,
    Firefox,
    Safari,
    Edge,
    Brave,
    Opera,
    Vivaldi,
    Whale,
}

impl FromStr for Browser {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "chrome" | "chromium" => Ok(Browser::Chrome),
            "firefox" => Ok(Browser::Firefox),
            "safari" => Ok(Browser::Safari),
            "edge" => Ok(Browser::Edge),
            "brave" => Ok(Browser::Brave),
            "opera" => Ok(Browser::Opera),
            "vivaldi" => Ok(Browser::Vivaldi),
            "whale" => Ok(Browser::Whale),
            _ => Err(()),
        }
    }
}

/// Browser cookie configuration
#[derive(Debug, Clone)]
pub struct BrowserCookieConfig {
    pub browser: Browser,
    pub profile: Option<String>,
    pub container: Option<String>,
    pub keyring: Option<String>,
}

impl BrowserCookieConfig {
    /// Parse from yt-dlp style format: BROWSER[+KEYRING][:PROFILE][::CONTAINER]
    pub fn parse(input: &str) -> Result<Self> {
        let mut parts = input.split("::");
        let browser_part = parts.next().unwrap_or(input);
        let container = parts.next().map(|s| s.to_string());

        let mut browser_profile_parts = browser_part.split(':');
        let browser_keyring_part = browser_profile_parts.next().unwrap();
        let profile = browser_profile_parts.next().map(|s| s.to_string());

        let mut browser_keyring_split = browser_keyring_part.split('+');
        let browser_str = browser_keyring_split.next().unwrap();
        let keyring = browser_keyring_split.next().map(|s| s.to_string());

        let browser = browser_str.parse::<Browser>().map_err(|_| {
            crate::error::RurlError::Config(format!("Unsupported browser: {}", browser_str))
        })?;

        Ok(BrowserCookieConfig {
            browser,
            profile,
            container,
            keyring,
        })
    }
}

/// HTTP method enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Patch,
    Trace,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let method = match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Trace => "TRACE",
        };
        write!(f, "{}", method)
    }
}

impl FromStr for HttpMethod {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "DELETE" => Ok(HttpMethod::Delete),
            "HEAD" => Ok(HttpMethod::Head),
            "OPTIONS" => Ok(HttpMethod::Options),
            "PATCH" => Ok(HttpMethod::Patch),
            "TRACE" => Ok(HttpMethod::Trace),
            _ => Err(()),
        }
    }
}

/// Proxy configuration
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// SSL/TLS configuration
#[derive(Debug, Clone)]
pub struct SslConfig {
    pub verify_certs: bool,
    pub ca_cert_file: Option<PathBuf>,
    pub client_cert_file: Option<PathBuf>,
    pub client_key_file: Option<PathBuf>,
}

/// Output configuration
#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub file: Option<PathBuf>,
    pub verbose: bool,
    pub silent: bool,
    pub show_progress: bool,
    pub format_json: bool,
}

/// Main configuration struct
#[derive(Debug, Clone)]
pub struct Config {
    pub url: String,
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub data: Option<String>,
    pub user_agent: Option<String>,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub retry_count: u32,
    pub retry_delay: Duration,
    pub browser_cookies: Option<BrowserCookieConfig>,
    pub proxy: Option<ProxyConfig>,
    pub ssl: SslConfig,
    pub output: OutputConfig,
    pub auth_username: Option<String>,
    pub auth_password: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            url: String::new(),
            method: HttpMethod::Get,
            headers: HashMap::new(),
            data: None,
            user_agent: Some(format!("rurl/{}", crate::VERSION)),
            follow_redirects: false,
            max_redirects: 50,
            timeout: Duration::from_secs(300),
            connect_timeout: Duration::from_secs(30),
            retry_count: 0,
            retry_delay: Duration::from_secs(1),
            browser_cookies: None,
            proxy: None,
            ssl: SslConfig {
                verify_certs: true,
                ca_cert_file: None,
                client_cert_file: None,
                client_key_file: None,
            },
            output: OutputConfig {
                file: None,
                verbose: false,
                silent: false,
                show_progress: true,
                format_json: false,
            },
            auth_username: None,
            auth_password: None,
        }
    }
}
