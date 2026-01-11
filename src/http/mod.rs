//! HTTP client module
//!
//! This module provides the core HTTP/HTTPS client functionality.

use crate::browser::BrowserCookieExtractor;
use crate::config::{Config, HttpMethod};
use crate::error::{Result, RurlError};
use reqwest::{Client, ClientBuilder, Method};
use url::Url;

pub mod auth;
pub mod request;
pub mod response;

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
            .redirect(if config.follow_redirects {
                reqwest::redirect::Policy::limited(config.max_redirects as usize)
            } else {
                reqwest::redirect::Policy::none()
            });

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
        let method = match self.config.method {
            HttpMethod::Get => Method::GET,
            HttpMethod::Post => Method::POST,
            HttpMethod::Put => Method::PUT,
            HttpMethod::Delete => Method::DELETE,
            HttpMethod::Head => Method::HEAD,
            HttpMethod::Options => Method::OPTIONS,
            HttpMethod::Patch => Method::PATCH,
            HttpMethod::Trace => Method::TRACE,
        };

        let mut request = self.client.request(method, &self.config.url);

        // Add headers
        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        // Add User-Agent
        if let Some(user_agent) = &self.config.user_agent {
            request = request.header("User-Agent", user_agent);
        }

        // Add authentication
        if let (Some(username), Some(password)) =
            (&self.config.auth_username, &self.config.auth_password)
        {
            request = request.basic_auth(username, Some(password));
        }

        // Add request body for POST/PUT/PATCH
        if let Some(data) = &self.config.data {
            request = request.body(data.clone());
        }

        if let Some(browser_config) = &self.config.browser_cookies {
            let extractor = BrowserCookieExtractor::new(browser_config.clone());
            let store = extractor.extract_cookies().await?;
            let url = Url::parse(&self.config.url).map_err(|e| {
                RurlError::InvalidUrl(format!("Invalid URL '{}': {}", self.config.url, e))
            })?;
            let cookies = extractor.cookies_for_url(&store, &url);
            if !cookies.is_empty() {
                let mut header_value = extractor.cookies_to_header(&cookies);
                if let Some(existing) = find_cookie_header(&self.config.headers) {
                    header_value = format!("{}; {}", existing, header_value);
                }
                request = request.header("Cookie", header_value);
            }
        }

        let request = request.build().map_err(RurlError::Http)?;

        if self.config.output.verbose && !self.config.output.silent {
            write_verbose_request_headers(&request);
        }

        self.client.execute(request).await.map_err(RurlError::Http)
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
