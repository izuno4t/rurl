//! Output formatting and display utilities

use crate::config::OutputConfig;
use crate::error::Result;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

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

/// Progress indicator for long-running operations
pub struct ProgressIndicator {
    enabled: bool,
}

impl ProgressIndicator {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Show progress for download/upload operations
    pub fn show_progress(&self, _current: u64, _total: Option<u64>) {
        if self.enabled {
            // Implementation for progress bar would go here
            // Could use indicatif crate for fancy progress bars
        }
    }
}
