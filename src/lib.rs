//! rurl - A modern curl alternative written in Rust
//!
//! This crate provides a curl-compatible HTTP client with native browser
//! cookie integration, memory safety guarantees, and modern async/await patterns.

pub mod browser;
pub mod cli;
pub mod config;
pub mod error;
pub mod http;
pub mod output;
pub mod ssl;
pub mod utils;

pub use error::{Result, RurlError};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");