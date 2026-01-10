//! CLI argument parsing module
//!
//! This module handles command-line argument parsing and application entry point.

use crate::config::{Config, BrowserCookieConfig, HttpMethod};
use crate::error::{Result, RurlError};
use crate::http::HttpClient;
use crate::utils::{UrlUtils, StringUtils, FileUtils};
use clap::{Arg, Command, ArgMatches};

pub mod args;
pub mod runner;

/// Main entry point for the CLI application
pub fn run() {
    env_logger::init();
    
    let app = create_app();
    let matches = app.get_matches();
    
    match run_with_args(&matches) {
        Ok(()) => {},
        Err(e) => {
            eprintln!("rurl: error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Run rurl with parsed command line arguments
fn run_with_args(matches: &ArgMatches) -> Result<()> {
    let config = build_config_from_args(matches)?;
    
    // Create HTTP client and execute request
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| RurlError::Config(format!("Failed to create async runtime: {}", e)))?;
    
    rt.block_on(async {
        let client = HttpClient::new(config)?;
        let response = client.execute().await?;
        
        // Handle response output
        let body = response.text().await
            .map_err(|e| RurlError::Http(e))?;
            
        println!("{}", body);
        Ok(())
    })
}

/// Create the CLI application structure
fn create_app() -> Command {
    Command::new("rurl")
        .version(crate::VERSION)
        .about("A modern curl alternative with browser cookie support")
        .arg(Arg::new("url")
            .help("The URL to request")
            .required(true)
            .index(1))
        .arg(Arg::new("request")
            .short('X')
            .long("request")
            .value_name("METHOD")
            .help("HTTP request method")
            .default_value("GET"))
        .arg(Arg::new("header")
            .short('H')
            .long("header")
            .value_name("HEADER")
            .help("Add custom HTTP header")
            .action(clap::ArgAction::Append))
        .arg(Arg::new("data")
            .short('d')
            .long("data")
            .value_name("DATA")
            .help("HTTP POST data"))
        .arg(Arg::new("cookies-from-browser")
            .long("cookies-from-browser")
            .value_name("BROWSER[+KEYRING][:PROFILE][::CONTAINER]")
            .help("Extract cookies from browser"))
        .arg(Arg::new("verbose")
            .short('v')
            .long("verbose")
            .help("Verbose output")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("silent")
            .short('s')
            .long("silent")
            .help("Silent mode")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("output")
            .short('o')
            .long("output")
            .value_name("FILE")
            .help("Write output to file"))
        .arg(Arg::new("user")
            .short('u')
            .long("user")
            .value_name("USER[:PASSWORD]")
            .help("HTTP authentication"))
        .arg(Arg::new("proxy")
            .short('x')
            .long("proxy")
            .value_name("[PROTOCOL://]HOST[:PORT]")
            .help("Use proxy server"))
        .arg(Arg::new("insecure")
            .short('k')
            .long("insecure")
            .help("Allow insecure SSL connections")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("location")
            .short('L')
            .long("location")
            .help("Follow redirects")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("user-agent")
            .short('A')
            .long("user-agent")
            .value_name("STRING")
            .help("User-Agent header"))
        .arg(Arg::new("timeout")
            .long("timeout")
            .value_name("SECONDS")
            .help("Maximum time for operation")
            .default_value("300"))
        .arg(Arg::new("connect-timeout")
            .long("connect-timeout")
            .value_name("SECONDS")
            .help("Maximum time for connection")
            .default_value("30"))
        .arg(Arg::new("cacert")
            .long("cacert")
            .value_name("FILE")
            .help("CA certificate bundle file"))
        .arg(Arg::new("cert")
            .long("cert")
            .value_name("FILE")
            .help("Client certificate file"))
        .arg(Arg::new("key")
            .long("key")
            .value_name("FILE")
            .help("Private key file"))
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
        config.method = HttpMethod::from_str(method_str)
            .ok_or_else(|| RurlError::Config(format!("Unknown HTTP method: {}", method_str)))?;
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
    
    // Configure output
    config.output.verbose = matches.get_flag("verbose");
    config.output.silent = matches.get_flag("silent");
    
    if let Some(output_file) = matches.get_one::<String>("output") {
        config.output.file = Some(FileUtils::expand_path(output_file)?);
    }
    
    // Configure redirects
    config.follow_redirects = matches.get_flag("location");
    
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
    
    // Configure User-Agent
    if let Some(user_agent) = matches.get_one::<String>("user-agent") {
        config.user_agent = Some(user_agent.clone());
    }
    
    Ok(config)
}
