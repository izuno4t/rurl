.PHONY: fmt fmt-check clippy-all-target clippy-no-deps check test build lint verify verify-release

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy-all-target:
	cargo clippy --all-targets -- -D warnings

clippy-no-deps:
	cargo clippy --all-targets --no-deps -- -D warnings

check:
	cargo check --all-targets

test:
	cargo test --all

build:
	cargo build

lint: fmt-check clippy-no-deps

verify: fmt-check clippy-no-deps check test

verify-release: fmt-check clippy-all-target check test

all: verify build
