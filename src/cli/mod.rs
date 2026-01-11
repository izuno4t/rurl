//! CLI argument parsing module
//!
//! This module handles command-line argument parsing and application entry point.

use crate::config::{BrowserCookieConfig, Config, HttpMethod, ProxyConfig};
use crate::error::{Result, RurlError};
use crate::exit_code::exit_code_for_error;
use crate::http::HttpClient;
use crate::i18n::localize_error;
use crate::output::OutputManager;
use crate::utils::{FileUtils, StringUtils, UrlUtils};
use clap::{Arg, ArgMatches, Command};
use log::{error, info};

pub mod args;
pub mod runner;

/// Main entry point for the CLI application
pub fn run() {
    let app = create_app();
    let matches = app.get_matches();
    let silent = matches.get_flag("silent");

    match run_with_args(&matches) {
        Ok(()) => {}
        Err(e) => {
            error!("request failed: {}", e);
            if !silent {
                eprintln!("rurl: {}", localize_error(&e));
            }
            std::process::exit(exit_code_for_error(&e));
        }
    }
}

/// Run rurl with parsed command line arguments
fn run_with_args(matches: &ArgMatches) -> Result<()> {
    let config = build_config_from_args(matches)?;
    info!("request: {} {}", config.method, config.url);

    // Create HTTP client and execute request
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| RurlError::Config(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let output_config = config.output.clone();
        let client = HttpClient::new(config)?;
        let response_history = client.execute_with_history().await?;
        let output = OutputManager::new(output_config);
        output
            .write_response(response_history.response, &response_history.chain)
            .await?;
        Ok(())
    })
}

/// Create the CLI application structure
fn create_app() -> Command {
    Command::new("rurl")
        .version(crate::VERSION)
        .about("A modern curl alternative with browser cookie support")
        .arg(
            Arg::new("url")
                .help("The URL to request")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("request")
                .short('X')
                .long("request")
                .value_name("METHOD")
                .help("HTTP request method"),
        )
        .arg(
            Arg::new("header")
                .short('H')
                .long("header")
                .value_name("HEADER")
                .help("Add custom HTTP header")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("data")
                .short('d')
                .long("data")
                .value_name("DATA")
                .help("HTTP POST data"),
        )
        .arg(
            Arg::new("cookies-from-browser")
                .long("cookies-from-browser")
                .value_name("BROWSER[+KEYRING][:PROFILE][::CONTAINER]")
                .help("Extract cookies from browser"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Verbose output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("silent")
                .short('s')
                .long("silent")
                .help("Silent mode")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("pretty-json")
                .long("pretty-json")
                .help("Pretty-print JSON responses")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-progress-meter")
                .long("no-progress-meter")
                .help("Disable progress meter")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Write output to file"),
        )
        .arg(
            Arg::new("user")
                .short('u')
                .long("user")
                .value_name("USER[:PASSWORD]")
                .help("HTTP authentication"),
        )
        .arg(
            Arg::new("proxy")
                .short('x')
                .long("proxy")
                .value_name("[PROTOCOL://]HOST[:PORT]")
                .help("Use proxy server"),
        )
        .arg(
            Arg::new("proxy-user")
                .long("proxy-user")
                .value_name("USER[:PASSWORD]")
                .help("Proxy authentication"),
        )
        .arg(
            Arg::new("insecure")
                .short('k')
                .long("insecure")
                .help("Allow insecure SSL connections")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("location")
                .short('L')
                .long("location")
                .help("Follow redirects")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("location-trusted")
                .long("location-trusted")
                .help("Follow redirects and send credentials to other hosts")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("include")
                .short('i')
                .long("include")
                .help("Include response headers in output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("max-redirs")
                .long("max-redirs")
                .value_name("NUMBER")
                .help("Maximum number of redirects to follow (-1 for unlimited)"),
        )
        .arg(
            Arg::new("post301")
                .long("post301")
                .help("Do not switch POST to GET after 301")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("post302")
                .long("post302")
                .help("Do not switch POST to GET after 302")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("post303")
                .long("post303")
                .help("Do not switch POST to GET after 303")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("user-agent")
                .short('A')
                .long("user-agent")
                .value_name("STRING")
                .help("User-Agent header"),
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .value_name("SECONDS")
                .help("Maximum time for operation")
                .default_value("300"),
        )
        .arg(
            Arg::new("connect-timeout")
                .long("connect-timeout")
                .value_name("SECONDS")
                .help("Maximum time for connection")
                .default_value("30"),
        )
        .arg(
            Arg::new("retry")
                .long("retry")
                .value_name("NUMBER")
                .help("Number of retry attempts"),
        )
        .arg(
            Arg::new("retry-delay")
                .long("retry-delay")
                .value_name("SECONDS")
                .help("Delay between retries"),
        )
        .arg(
            Arg::new("cacert")
                .long("cacert")
                .value_name("FILE")
                .help("CA certificate bundle file"),
        )
        .arg(
            Arg::new("cert")
                .long("cert")
                .value_name("FILE")
                .help("Client certificate file"),
        )
        .arg(
            Arg::new("key")
                .long("key")
                .value_name("FILE")
                .help("Private key file"),
        )
}

/// Build configuration from command line arguments
fn build_config_from_args(matches: &ArgMatches) -> Result<Config> {
    let mut config = Config::default();

    // Parse URL
    if let Some(url_str) = matches.get_one::<String>("url") {
        let url = UrlUtils::validate_url(url_str)?;
        config.url = url.to_string();
    }

    // Parse HTTP method
    if let Some(method_str) = matches.get_one::<String>("request") {
        config.method = method_str
            .parse::<HttpMethod>()
            .map_err(|_| RurlError::Config(format!("Unknown HTTP method: {}", method_str)))?;
        config.request_method_explicit = true;
    }

    // Parse headers
    if let Some(headers) = matches.get_many::<String>("header") {
        for header_str in headers {
            let (key, value) = StringUtils::parse_header(header_str)?;
            config.headers.insert(key, value);
        }
    }

    // Parse data
    if let Some(data) = matches.get_one::<String>("data") {
        config.data = Some(data.clone());
    }
    if !config.request_method_explicit && config.data.is_some() {
        config.method = HttpMethod::Post;
    }

    // Parse browser cookies
    if let Some(browser_str) = matches.get_one::<String>("cookies-from-browser") {
        config.browser_cookies = Some(BrowserCookieConfig::parse(browser_str)?);
    }

    // Parse authentication
    if let Some(user_str) = matches.get_one::<String>("user") {
        let (username, password) = StringUtils::parse_header(&user_str.replace(':', ": "))?;
        config.auth_username = Some(username);
        config.auth_password = Some(password);
    }

    // Configure proxy
    let proxy_user = matches.get_one::<String>("proxy-user");
    if let Some(proxy_url) = matches.get_one::<String>("proxy") {
        let proxy_url = if proxy_url.contains("://") {
            proxy_url.clone()
        } else {
            format!("http://{}", proxy_url)
        };
        let mut proxy_config = ProxyConfig {
            url: proxy_url,
            username: None,
            password: None,
        };

        if let Some(proxy_user_str) = proxy_user {
            let mut parts = proxy_user_str.splitn(2, ':');
            let username = parts.next().unwrap_or_default();
            if username.is_empty() {
                return Err(RurlError::Config(
                    "Proxy user must include a username".to_string(),
                ));
            }
            proxy_config.username = Some(username.to_string());
            proxy_config.password = parts.next().map(|s| s.to_string());
        }

        config.proxy = Some(proxy_config);
    } else if proxy_user.is_some() {
        return Err(RurlError::Config(
            "Proxy user provided without proxy".to_string(),
        ));
    }

    // Configure output
    config.output.verbose = matches.get_flag("verbose");
    config.output.silent = matches.get_flag("silent");
    config.output.include_headers = matches.get_flag("include");
    config.output.format_json = matches.get_flag("pretty-json");
    config.output.show_progress = !matches.get_flag("no-progress-meter");
    if config.output.silent {
        config.output.show_progress = false;
    }

    if let Some(output_file) = matches.get_one::<String>("output") {
        config.output.file = Some(FileUtils::expand_path(output_file)?);
    }

    // Configure redirects
    config.follow_redirects = matches.get_flag("location");
    config.location_trusted = matches.get_flag("location-trusted");
    if config.location_trusted {
        config.follow_redirects = true;
    }
    config.post301 = matches.get_flag("post301");
    config.post302 = matches.get_flag("post302");
    config.post303 = matches.get_flag("post303");
    if let Some(max_redirs_str) = matches.get_one::<String>("max-redirs") {
        if max_redirs_str.trim() == "-1" {
            config.max_redirects = None;
        } else {
            let value = max_redirs_str.parse::<i64>().map_err(|_| {
                RurlError::Config(format!("Invalid max-redirs value: {}", max_redirs_str))
            })?;
            if value < 0 {
                return Err(RurlError::Config(format!(
                    "Invalid max-redirs value: {}",
                    max_redirs_str
                )));
            }
            config.max_redirects = Some(value as usize);
        }
    }

    // Configure SSL
    config.ssl.verify_certs = !matches.get_flag("insecure");

    if let Some(cacert_file) = matches.get_one::<String>("cacert") {
        config.ssl.ca_cert_file = Some(FileUtils::expand_path(cacert_file)?);
    }

    if let Some(cert_file) = matches.get_one::<String>("cert") {
        config.ssl.client_cert_file = Some(FileUtils::expand_path(cert_file)?);
    }

    if let Some(key_file) = matches.get_one::<String>("key") {
        config.ssl.client_key_file = Some(FileUtils::expand_path(key_file)?);
    }

    // Configure timeouts
    if let Some(timeout_str) = matches.get_one::<String>("timeout") {
        config.timeout = StringUtils::parse_timeout(timeout_str)?;
    }

    if let Some(connect_timeout_str) = matches.get_one::<String>("connect-timeout") {
        config.connect_timeout = StringUtils::parse_timeout(connect_timeout_str)?;
    }

    // Configure retries
    if let Some(retry_str) = matches.get_one::<String>("retry") {
        config.retry_count = retry_str
            .parse::<u32>()
            .map_err(|_| RurlError::Config(format!("Invalid retry value: {}", retry_str)))?;
    }

    if let Some(retry_delay_str) = matches.get_one::<String>("retry-delay") {
        config.retry_delay = StringUtils::parse_timeout(retry_delay_str)?;
    }

    // Configure User-Agent
    if let Some(user_agent) = matches.get_one::<String>("user-agent") {
        config.user_agent = Some(user_agent.clone());
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::{build_config_from_args, create_app};
    use crate::config::HttpMethod;

    fn matches_from(args: &[&str]) -> clap::ArgMatches {
        create_app().try_get_matches_from(args).expect("matches")
    }

    #[test]
    fn build_config_sets_defaults_from_url() {
        let matches = matches_from(&["rurl", "example.com"]);
        let config = build_config_from_args(&matches).expect("config");
        assert_eq!(config.url, "http://example.com/");
        assert_eq!(config.method, HttpMethod::Get);
    }

    #[test]
    fn build_config_sets_post_when_data_present() {
        let matches = matches_from(&["rurl", "http://example.com", "-d", "a=1"]);
        let config = build_config_from_args(&matches).expect("config");
        assert_eq!(config.method, HttpMethod::Post);
        assert_eq!(config.data.as_deref(), Some("a=1"));
        assert!(!config.request_method_explicit);
    }

    #[test]
    fn build_config_respects_explicit_method() {
        let matches = matches_from(&["rurl", "http://example.com", "-X", "PUT"]);
        let config = build_config_from_args(&matches).expect("config");
        assert_eq!(config.method, HttpMethod::Put);
        assert!(config.request_method_explicit);
    }

    #[test]
    fn build_config_parses_headers() {
        let matches = matches_from(&["rurl", "http://example.com", "-H", "X-Test: value"]);
        let config = build_config_from_args(&matches).expect("config");
        assert_eq!(
            config.headers.get("X-Test").map(String::as_str),
            Some("value")
        );
    }

    #[test]
    fn build_config_supports_redirect_flags() {
        let matches = matches_from(&["rurl", "http://example.com", "--location-trusted"]);
        let config = build_config_from_args(&matches).expect("config");
        assert!(config.follow_redirects);
        assert!(config.location_trusted);
    }

    #[test]
    fn build_config_handles_max_redirs_unlimited() {
        let matches = matches_from(&["rurl", "http://example.com", "--max-redirs=-1"]);
        let config = build_config_from_args(&matches).expect("config");
        assert!(config.max_redirects.is_none());
    }

    #[test]
    fn build_config_proxy_user_requires_proxy() {
        let matches = matches_from(&["rurl", "http://example.com", "--proxy-user", "user:pass"]);
        let err = build_config_from_args(&matches).expect_err("proxy error");
        assert!(err.to_string().contains("Proxy user"));
    }

    #[test]
    fn build_config_proxy_user_requires_username() {
        let matches = matches_from(&[
            "rurl",
            "http://example.com",
            "--proxy",
            "http://proxy",
            "--proxy-user",
            ":pass",
        ]);
        let err = build_config_from_args(&matches).expect_err("proxy error");
        assert!(err.to_string().contains("username"));
    }

    #[test]
    fn build_config_silent_disables_progress() {
        let matches = matches_from(&[
            "rurl",
            "http://example.com",
            "--silent",
            "--no-progress-meter",
        ]);
        let config = build_config_from_args(&matches).expect("config");
        assert!(config.output.silent);
        assert!(!config.output.show_progress);
    }
}
