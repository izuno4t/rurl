.PHONY: fmt fmt-check clippy check test build lint verify

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets -- -D warnings

check:
	cargo check --all-targets

test:
	cargo test --all

build:
	cargo build

lint: fmt-check clippy

verify: lint check test
