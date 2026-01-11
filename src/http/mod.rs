//! HTTP client module
//!
//! This module provides the core HTTP/HTTPS client functionality.

use crate::browser::BrowserCookieExtractor;
use crate::config::{Config, HttpMethod};
use crate::error::{Result, RurlError};
use reqwest::header::{LOCATION, RETRY_AFTER};
use reqwest::{Client, ClientBuilder, Method};
use std::time::Duration;
use url::Url;

pub mod auth;
pub mod request;
pub mod response;

pub use response::{ResponseHistory, ResponseInfo};

/// HTTP client wrapper
pub struct HttpClient {
    client: Client,
    config: Config,
}

impl HttpClient {
    /// Create a new HTTP client with the given configuration
    pub fn new(config: Config) -> Result<Self> {
        let mut builder = ClientBuilder::new()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .redirect(reqwest::redirect::Policy::none());

        // Configure proxy if specified
        if let Some(proxy_config) = &config.proxy {
            let proxy = reqwest::Proxy::all(&proxy_config.url)
                .map_err(|e| RurlError::Proxy(format!("Invalid proxy: {}", e)))?;

            let proxy = if let (Some(username), Some(password)) =
                (&proxy_config.username, &proxy_config.password)
            {
                proxy.basic_auth(username, password)
            } else {
                proxy
            };

            builder = builder.proxy(proxy);
        }

        // Configure SSL/TLS
        if !config.ssl.verify_certs {
            builder = builder.danger_accept_invalid_certs(true);
        }

        let client = builder.build().map_err(RurlError::Http)?;

        Ok(Self { client, config })
    }

    /// Execute an HTTP request
    pub async fn execute(&self) -> Result<reqwest::Response> {
        Ok(self.execute_with_history().await?.response)
    }

    pub async fn execute_with_history(&self) -> Result<ResponseHistory> {
        let mut retries_left = self.config.retry_count;
        loop {
            let result = self.execute_with_history_once().await;
            match result {
                Ok(history) => {
                    if retries_left == 0 {
                        return Ok(history);
                    }
                    if let Some(delay) = retry_delay_from_response(
                        history.response.status(),
                        history.response.headers(),
                        self.config.retry_delay,
                    ) {
                        if delay > Duration::from_millis(0) {
                            tokio::time::sleep(delay).await;
                        }
                        retries_left -= 1;
                        continue;
                    }
                    return Ok(history);
                }
                Err(err) => {
                    if retries_left == 0 || !should_retry_error(&err) {
                        return Err(err);
                    }
                    if self.config.retry_delay > Duration::from_millis(0) {
                        tokio::time::sleep(self.config.retry_delay).await;
                    }
                    retries_left -= 1;
                }
            }
        }
    }

    async fn execute_with_history_once(&self) -> Result<ResponseHistory> {
        let mut history = Vec::new();
        let mut current_url = Url::parse(&self.config.url).map_err(|e| {
            RurlError::InvalidUrl(format!("Invalid URL '{}': {}", self.config.url, e))
        })?;
        let initial_origin = redirect_origin_key(&current_url);
        let mut current_method = self.config.method.clone();
        let mut current_data = self.config.data.clone();
        let mut redirects_followed = 0usize;
        let cookie_context = if let Some(browser_config) = &self.config.browser_cookies {
            let extractor = BrowserCookieExtractor::new(browser_config.clone());
            let store = extractor.extract_cookies().await?;
            Some((extractor, store))
        } else {
            None
        };

        loop {
            let method = match current_method {
                HttpMethod::Get => Method::GET,
                HttpMethod::Post => Method::POST,
                HttpMethod::Put => Method::PUT,
                HttpMethod::Delete => Method::DELETE,
                HttpMethod::Head => Method::HEAD,
                HttpMethod::Options => Method::OPTIONS,
                HttpMethod::Patch => Method::PATCH,
                HttpMethod::Trace => Method::TRACE,
            };
            let same_origin = redirect_origin_key(&current_url) == initial_origin;

            let mut request = self.client.request(method, current_url.as_str());

            // Add headers
            for (key, value) in &self.config.headers {
                if !same_origin && is_sensitive_header(key) && !self.config.location_trusted {
                    continue;
                }
                request = request.header(key, value);
            }

            // Add User-Agent
            if let Some(user_agent) = &self.config.user_agent {
                request = request.header("User-Agent", user_agent);
            }

            // Add authentication
            if same_origin || self.config.location_trusted {
                if let (Some(username), Some(password)) =
                    (&self.config.auth_username, &self.config.auth_password)
                {
                    request = request.basic_auth(username, Some(password));
                }
            }

            // Add request body for POST/PUT/PATCH
            if let Some(data) = &current_data {
                request = request.body(data.clone());
            }

            if let Some((extractor, store)) = &cookie_context {
                let cookies = extractor.cookies_for_url(store, &current_url);
                if !cookies.is_empty() {
                    let mut header_value = extractor.cookies_to_header(&cookies);
                    let existing = if same_origin || self.config.location_trusted {
                        find_cookie_header(&self.config.headers)
                    } else {
                        None
                    };
                    if let Some(existing) = existing {
                        header_value = format!("{}; {}", existing, header_value);
                    }
                    request = request.header("Cookie", header_value);
                }
            }

            let request = request.build().map_err(RurlError::Http)?;

            if self.config.output.verbose && !self.config.output.silent {
                write_verbose_request_headers(&request);
            }

            let response = self
                .client
                .execute(request)
                .await
                .map_err(RurlError::Http)?;
            let status = response.status();
            let info = ResponseInfo {
                version: response.version(),
                status,
                headers: response.headers().clone(),
            };
            history.push(info);

            if !self.config.follow_redirects || !status.is_redirection() {
                return Ok(ResponseHistory {
                    response,
                    chain: history,
                });
            }

            let location = match response.headers().get(LOCATION) {
                Some(value) => value,
                None => {
                    return Ok(ResponseHistory {
                        response,
                        chain: history,
                    })
                }
            };
            let location_str = location.to_str().map_err(|_| {
                RurlError::InvalidUrl("Redirect location contains invalid characters".to_string())
            })?;

            if let Some(limit) = self.config.max_redirects {
                if redirects_followed >= limit {
                    return Err(RurlError::RedirectLimitExceeded(limit));
                }
            }
            redirects_followed += 1;

            let next_url = current_url.join(location_str).map_err(|e| {
                RurlError::InvalidUrl(format!("Invalid redirect URL '{}': {}", location_str, e))
            })?;

            if !self.config.request_method_explicit {
                let status_code = status.as_u16();
                if current_method == HttpMethod::Post {
                    let keep_post = match status_code {
                        301 => self.config.post301,
                        302 => self.config.post302,
                        303 => self.config.post303,
                        _ => false,
                    };
                    if matches!(status_code, 301..=303) && !keep_post {
                        current_method = HttpMethod::Get;
                        current_data = None;
                    }
                } else if status_code == 303 && current_method != HttpMethod::Get {
                    current_method = HttpMethod::Get;
                    current_data = None;
                }
            }
            current_url = next_url;
        }
    }
}

fn find_cookie_header(headers: &std::collections::HashMap<String, String>) -> Option<String> {
    for (key, value) in headers {
        if key.eq_ignore_ascii_case("cookie") {
            return Some(value.clone());
        }
    }
    None
}

fn is_sensitive_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("authorization") || name.eq_ignore_ascii_case("cookie")
}

fn redirect_origin_key(url: &Url) -> (String, Option<u16>) {
    (url.host_str().unwrap_or_default().to_string(), url.port())
}

fn write_verbose_request_headers(request: &reqwest::Request) {
    let url = request.url();
    let path = request_path(url);
    eprintln!("> {} {}", request.method(), path);

    if let Some(host_value) = request.headers().get("host") {
        let host = host_value.to_str().unwrap_or("<non-utf8>");
        eprintln!("> Host: {}", host);
    } else if let Some(host) = url.host_str() {
        let host = match url.port() {
            Some(port) => format!("{}:{}", host, port),
            None => host.to_string(),
        };
        eprintln!("> Host: {}", host);
    }

    for (name, value) in request.headers().iter() {
        if name.as_str().eq_ignore_ascii_case("host") {
            continue;
        }
        let value = value.to_str().unwrap_or("<non-utf8>");
        eprintln!("> {}: {}", name, value);
    }
    eprintln!(">");
}

fn request_path(url: &Url) -> String {
    match url[url::Position::BeforePath..].trim() {
        "" => "/".to_string(),
        path => path.to_string(),
    }
}

fn should_retry_error(err: &RurlError) -> bool {
    match err {
        RurlError::Http(http_err) => http_err.is_timeout() || http_err.is_connect(),
        _ => false,
    }
}

fn retry_delay_from_response(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    default_delay: Duration,
) -> Option<Duration> {
    let is_retryable = matches!(
        status.as_u16(),
        408 | 429 | 500 | 502 | 503 | 504 | 522 | 524
    );
    if !is_retryable {
        return None;
    }
    let mut delay = default_delay;
    if let Some(value) = headers.get(RETRY_AFTER) {
        if let Ok(text) = value.to_str() {
            if let Ok(seconds) = text.parse::<u64>() {
                let retry_after = Duration::from_secs(seconds);
                if retry_after > delay {
                    delay = retry_after;
                }
            }
        }
    }
    Some(delay)
}

#[cfg(test)]
mod tests {
    use super::redirect_origin_key;
    use url::Url;

    #[test]
    fn redirect_origin_key_ignores_scheme() {
        let http = Url::parse("http://example.com/path").expect("valid url");
        let https = Url::parse("https://example.com/path").expect("valid url");
        assert_eq!(redirect_origin_key(&http), redirect_origin_key(&https));
    }

    #[test]
    fn redirect_origin_key_respects_explicit_port() {
        let http = Url::parse("http://example.com:8080/path").expect("valid url");
        let https = Url::parse("https://example.com:8443/path").expect("valid url");
        assert_ne!(redirect_origin_key(&http), redirect_origin_key(&https));
    }
}
