# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Mandatory Pre-Work Checks

**Before starting any work:**

1. **Always review docs/WORKING_PRINCIPLES.md** and follow its standards throughout your work
2. **Always check docs/TASK.md** to verify current task status and ensure alignment with project state
3. **If task status doesn't match actual work progress, update docs/TASK.md immediately** to maintain accuracy

## Project Overview

rurl is a modern curl alternative written in Rust with native browser cookie integration. The project aims to provide curl-compatible functionality while adding browser session support for authenticated requests.

## Core Architecture

### Module Structure
The codebase is organized around these core modules:

- **`config`**: Central configuration system handling browser selection, HTTP options, SSL settings, and CLI argument parsing. Implements yt-dlp-style browser specification format `BROWSER[+KEYRING][:PROFILE][::CONTAINER]`
- **`browser`**: Cross-platform browser cookie extraction supporting Chrome/Chromium, Firefox, Safari, Edge, and other browsers. Each browser has its own submodule with OS-specific implementations
- **`http`**: Async HTTP client built on reqwest with request/response handling, authentication helpers, and curl-compatible features
- **`cli`**: Command-line interface using clap with curl-compatible argument structure
- **`error`**: Centralized error handling using thiserror with specific error types for different failure modes
- **`ssl`**: SSL/TLS certificate handling and validation utilities
- **`output`**: Response formatting, progress indication, and file output handling
- **`utils`**: Shared utilities for URL parsing, file operations, and string processing

### Key Design Patterns
- **Configuration-driven architecture**: The `Config` struct serves as the central data structure passed between modules
- **Platform-specific compilation**: Uses conditional compilation (`#[cfg()]`) for OS-specific browser cookie extraction
- **Error propagation**: Custom `Result<T>` type alias with `RurlError` for consistent error handling
- **Async-first**: Built around tokio runtime for HTTP operations

## Development Commands

### Building and Testing
```bash
# Basic development workflow
cargo check                    # Fast compilation check
cargo build                    # Development build
cargo build --release          # Optimized release build
cargo test                     # Run all tests
cargo test basic               # Run specific test

# Code quality
cargo fmt --all -- --check     # Check formatting
cargo clippy --all-targets -- -D warnings  # Lint code

# Install locally for testing
cargo install --path .
```

### Browser Cookie Integration
The project uses a placeholder for the `rookie` crate (currently commented out in Cargo.toml) for browser cookie extraction. When implementing browser features, uncomment this dependency and integrate with the browser modules.

## Task Management

The project follows a strict task-based development approach documented in `docs/TASK.md`. Key phases:
1. **Phase 1**: Project foundation (T001-T004) - Currently in progress
2. **Phase 2**: HTTP basic functionality (T005-T009)
3. **Phase 3**: Browser cookie integration (T010-T018) - Core differentiator

Always update task status in `docs/TASK.md` when working on features.

## Working Principles

This repository enforces strict working principles documented in `docs/WORKING_PRINCIPLES.md`:

- **Planning required**: Always create a plan before implementation with clear goals and scope
- **Research methodology**: Use official documentation as primary sources, validate with multiple sources
- **Information source documentation**: Include URLs and reliability level when citing research
- **No assumptions**: Clarify ambiguities rather than making assumptions
- **Task tracking**: Update task status and document decisions

## CI/CD Integration

The project has GitHub Actions workflows for:
- **Basic build check** (`basic.yml`): Simple compilation and test verification
- **Full CI** (`ci.yml`): Multi-platform builds, formatting, linting, and comprehensive testing

Rust version requirement: 1.92+ (specified in `rust-version` field)

**Local Development Constraints**: If using older Rust versions locally, rely on GitHub Actions for build verification and quality assurance. The CI pipeline is designed to handle the complete development workflow.

## Browser Cookie Implementation Notes

The core value proposition is browser cookie integration. The architecture supports:
- **Multi-browser support**: Chrome/Chromium family, Firefox, Safari (macOS), Edge
- **Profile specification**: Follows yt-dlp format for browser/profile selection
- **Cross-platform**: Different cookie storage mechanisms per OS
- **Security considerations**: Handles encrypted cookie stores and keyring integration

When implementing browser features, focus on the `BrowserCookieConfig::parse()` method and browser-specific extraction in the `browser/` module.
