# Repository Guidelines

## Agent Execution Rules

### IMPORTANT - READ BEFORE STARTING ANY TASK

This repository enforces common working principles and Rust coding conventions.
Before performing any task, you MUST do the following:

1. Read and understand:
   - `WORKING_PRINCIPLES.md`
   - `RUST_CODING_GUIDELINES.md`
   - `docs/TASK-1.1.0.md`
2. Confirm that your planned actions comply with these documents.
3. If task status does not match actual progress, update `docs/TASK-1.1.0.md`
   immediately to maintain accuracy.
4. If a rule cannot be followed:
   - Explicitly state **which rule**
   - Explain **why**
   - Describe **the impact**
   - Obtain confirmation before proceeding.

Do NOT:

- Start implementation without a plan
- Make assumptions without verification
- Change public APIs without explicit instruction
- Bypass rules silently

Failure to follow these rules is considered incorrect task execution.

## Communication Language

Use English for reading project documents, but keep all interaction and
responses in Japanese.

## Specification Alignment

- When choosing between specifications or behaviors, confirm curl or yt-dlp
  behavior and align with it.
- If it remains unclear after confirmation, ask for a decision before
  proceeding.
- Spec alignment is based on curl; yt-dlp is for implementation reference only.
- Rationale: users are accustomed to curl behavior, so compatibility is prioritized.
Do not ask whether to align with curl. Only ask when there is a concrete reason
or unavoidable ambiguity that requires a decision.

Reference sources:

- <https://github.com/curl/curl>
- <https://github.com/yt-dlp/yt-dlp>

## Clippy Policy

- Release checks run `clippy` with dependencies.
- If dependency warnings fail clippy, resolve by updating or pinning
  dependencies; otherwise, stop the release.

## Project Structure & Module Organization

- `src/`: Rust crate source. Entry points are `src/main.rs` (CLI) and
  `src/lib.rs` (library). Core modules include `src/browser/`, `src/http/`,
  `src/cli/`, `src/config.rs`, and `src/error.rs`.
- `tests/`: Integration tests (example: `tests/basic.rs`).
- `docs/`: Project process docs (`docs/WORKING_PRINCIPLES.md`, `docs/TASK-1.1.0.md`,
  `docs/LOCAL_DEVELOPMENT.md`).
- `target/`: Build artifacts (generated).

## Build, Test, and Development Commands

Use Cargo from the repo root:

- `cargo check`: Fast compile check.
- `cargo build`: Development build.
- `cargo build --release`: Optimized build.
- `cargo test`: Run tests; `cargo test basic` runs the basic test.
- `cargo fmt --all -- --check`: Format check.
- `cargo clippy --all-targets -- -D warnings`: Lint with warnings as errors.
- `make verify`: Run fmt-check, clippy, check, and tests in one go.

Local note: the documented MSRV is 1.92+, and CI uses 1.92. If your local
Rust is older, rely on CI for build/test verification.

## Coding Style & Naming Conventions

- Indentation: 4 spaces per Rust standard (rustfmt). Run `cargo fmt` before
  pushing.
- Prefer idiomatic Rust modules and clear error types (`thiserror` patterns in
  `src/error.rs`).
- File/module naming follows Rust conventions (snake_case filenames, CamelCase types).

## Testing Guidelines

- Framework: built-in Rust test harness via Cargo.
- Name tests clearly (e.g., `basic` in `tests/basic.rs`) and keep integration
  tests in `tests/`.
- Run `cargo test` locally when possible; otherwise, use CI as the source of truth.

## Commit & Pull Request Guidelines

- Commit style appears to follow Conventional Commits (e.g., `feat: Add ...`).
  Use `feat:`, `fix:`, `chore:`, etc.
- PRs should describe the change, link related tasks from `docs/TASK-1.1.0.md`, and
  note any CLI behavior changes.

## Development Workflow Requirements

- Always read `docs/WORKING_PRINCIPLES.md` and `docs/TASK-1.1.0.md` before starting
  work.
- For any Git operations, read and follow `docs/GIT_RULES.md`.
- Create a brief plan before implementation and update `docs/TASK-1.1.0.md` when task
  status changes.
- When adding browser-cookie features, keep the yt-dlp style spec in mind:
  `BROWSER[+KEYRING][:PROFILE][::CONTAINER]`.

## Completion Check

Before marking work as complete, ensure `make all` succeeds locally.
Define "normal" as the build passing locally.
When modifying Markdown files, run `markdownlint` on the changed files.
Do not ask for next steps; proceed with requested tasks until completion.
After making code changes, always run verification (at minimum `cargo test`)
and report the results before declaring completion.
