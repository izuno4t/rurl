//! Output formatting and display utilities

use crate::config::OutputConfig;
use crate::error::{Result, RurlError};
use crate::http::response::{ResponseFormatter, ResponseInfo};
use futures_util::StreamExt;
use reqwest::header::CONTENT_TYPE;
use reqwest::Response;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant};

/// Output writer that handles file vs stdout
pub struct OutputWriter {
    config: OutputConfig,
}

impl OutputWriter {
    pub fn new(config: OutputConfig) -> Self {
        Self { config }
    }

    /// Write content to configured output
    pub fn write(&self, content: &str) -> Result<()> {
        if let Some(file_path) = &self.config.file {
            self.write_to_file(content, file_path)
        } else {
            self.write_to_stdout(content)
        }
    }

    /// Write verbose information (if enabled)
    pub fn write_verbose(&self, message: &str) -> Result<()> {
        if self.config.verbose && !self.config.silent {
            eprintln!("* {}", message);
        }
        Ok(())
    }

    /// Write error message
    pub fn write_error(&self, message: &str) -> Result<()> {
        if !self.config.silent {
            eprintln!("rurl: error: {}", message);
        }
        Ok(())
    }

    fn write_to_file(&self, content: &str, file_path: &Path) -> Result<()> {
        let mut file = File::create(file_path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    fn write_to_stdout(&self, content: &str) -> Result<()> {
        io::stdout().write_all(content.as_bytes())?;
        Ok(())
    }
}

pub struct OutputManager {
    config: OutputConfig,
    writer: OutputWriter,
    formatter: ResponseFormatter,
}

impl OutputManager {
    pub fn new(config: OutputConfig) -> Self {
        let writer = OutputWriter::new(config.clone());
        let formatter = ResponseFormatter::new(config.format_json);
        Self {
            config,
            writer,
            formatter,
        }
    }

    pub async fn write_response(&self, response: Response, history: &[ResponseInfo]) -> Result<()> {
        if self.config.verbose && !self.config.silent {
            self.write_verbose_headers(history);
        }

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string());
        let body = self.read_body_with_progress(response).await?;
        let formatted = self.formatter.format(&body, content_type.as_deref())?;

        let output = if self.config.include_headers {
            let mut combined = String::new();
            for info in history {
                combined.push_str(&format_response_headers(
                    info.version,
                    info.status,
                    &info.headers,
                ));
            }
            combined.push_str(&formatted);
            combined
        } else {
            formatted
        };

        self.writer.write(&output)
    }

    fn write_verbose_headers(&self, history: &[ResponseInfo]) {
        for info in history {
            eprintln!("< {} {}", http_version_label(info.version), info.status);
            for (name, value) in info.headers.iter() {
                let value = value.to_str().unwrap_or("<non-utf8>");
                eprintln!("< {}: {}", name, value);
            }
            eprintln!("<");
        }
    }

    async fn read_body_with_progress(&self, response: Response) -> Result<String> {
        let total = response.content_length();
        let mut progress =
            ProgressReporter::new(self.config.show_progress && !self.config.silent, total);
        let mut stream = response.bytes_stream();
        let mut buffer = Vec::new();
        let mut current = 0u64;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(RurlError::Http)?;
            current = current.saturating_add(chunk.len() as u64);
            buffer.extend_from_slice(&chunk);
            progress.update(current);
        }

        progress.finish(current);
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }
}

struct ProgressReporter {
    enabled: bool,
    total: Option<u64>,
    last_update: Instant,
    rendered: bool,
}

impl ProgressReporter {
    fn new(enabled: bool, total: Option<u64>) -> Self {
        Self {
            enabled,
            total,
            last_update: Instant::now(),
            rendered: false,
        }
    }

    fn update(&mut self, current: u64) {
        if !self.enabled {
            return;
        }
        if self.last_update.elapsed() < Duration::from_millis(100) {
            return;
        }
        self.last_update = Instant::now();
        self.rendered = true;
        eprint!("{}", progress_line(current, self.total));
    }

    fn finish(&mut self, current: u64) {
        if !self.enabled {
            return;
        }
        if !self.rendered {
            eprint!("{}", progress_line(current, self.total));
        }
        eprintln!();
    }
}

fn progress_line(current: u64, total: Option<u64>) -> String {
    match total {
        Some(total) if total > 0 => {
            let percent = (current as f64 / total as f64) * 100.0;
            format!(
                "\r{} / {} bytes ({:.0}%)",
                current,
                total,
                percent.min(100.0)
            )
        }
        _ => format!("\r{} bytes", current),
    }
}

fn format_response_headers(
    version: reqwest::Version,
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("{} {}\n", http_version_label(version), status));
    for (name, value) in headers.iter() {
        let value = value.to_str().unwrap_or("<non-utf8>");
        output.push_str(&format!("{}: {}\n", name, value));
    }
    output.push('\n');
    output
}

fn http_version_label(version: reqwest::Version) -> &'static str {
    match version {
        reqwest::Version::HTTP_09 => "HTTP/0.9",
        reqwest::Version::HTTP_10 => "HTTP/1.0",
        reqwest::Version::HTTP_11 => "HTTP/1.1",
        reqwest::Version::HTTP_2 => "HTTP/2",
        reqwest::Version::HTTP_3 => "HTTP/3",
        _ => "HTTP/1.1",
    }
}
