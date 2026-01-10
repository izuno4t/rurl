# Repository Guidelines

## Mandatory Pre-Work Checks

**Before starting any work:**

1. **Always review docs/WORKING_PRINCIPLES.md** and follow its standards throughout your work
2. **Always check docs/TASK.md** to verify current task status and ensure alignment with project state
3. **If task status doesn't match actual work progress, update docs/TASK.md immediately** to maintain accuracy

## Communication Language
Use English for reading project documents, but keep all interaction and responses in Japanese.

## Project Structure & Module Organization
- `src/`: Rust crate source. Entry points are `src/main.rs` (CLI) and `src/lib.rs` (library). Core modules include `src/browser/`, `src/http/`, `src/cli/`, `src/config.rs`, and `src/error.rs`.
- `tests/`: Integration tests (example: `tests/basic.rs`).
- `docs/`: Project process docs (`docs/WORKING_PRINCIPLES.md`, `docs/TASK.md`, `docs/LOCAL_DEVELOPMENT.md`).
- `target/`: Build artifacts (generated).

## Build, Test, and Development Commands
Use Cargo from the repo root:
- `cargo check`: Fast compile check.
- `cargo build`: Development build.
- `cargo build --release`: Optimized build.
- `cargo test`: Run tests; `cargo test basic` runs the basic test.
- `cargo fmt --all -- --check`: Format check.
- `cargo clippy --all-targets -- -D warnings`: Lint with warnings as errors.

Local note: the documented MSRV is 1.80+, and CI uses 1.80. If your local Rust is older, rely on CI for build/test verification.

## Coding Style & Naming Conventions
- Indentation: 4 spaces per Rust standard (rustfmt). Run `cargo fmt` before pushing.
- Prefer idiomatic Rust modules and clear error types (`thiserror` patterns in `src/error.rs`).
- File/module naming follows Rust conventions (snake_case filenames, CamelCase types).

## Testing Guidelines
- Framework: built-in Rust test harness via Cargo.
- Name tests clearly (e.g., `basic` in `tests/basic.rs`) and keep integration tests in `tests/`.
- Run `cargo test` locally when possible; otherwise, use CI as the source of truth.

## Commit & Pull Request Guidelines
- Commit style appears to follow Conventional Commits (e.g., `feat: Add ...`). Use `feat:`, `fix:`, `chore:`, etc.
- PRs should describe the change, link related tasks from `docs/TASK.md`, and note any CLI behavior changes.

## Development Workflow Requirements
- Always read `docs/WORKING_PRINCIPLES.md` and `docs/TASK.md` before starting work.
- Create a brief plan before implementation and update `docs/TASK.md` when task status changes.
- When adding browser-cookie features, keep the yt-dlp style spec in mind: `BROWSER[+KEYRING][:PROFILE][::CONTAINER]`.
