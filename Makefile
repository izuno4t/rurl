.PHONY: fmt fmt-check clippy-all-target clippy-no-deps check test build lint verify verify-release coverage coverage-ci

TOOLCHAIN := $(shell awk -F'"' '/^channel/ {print $$2}' rust-toolchain.toml)

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

verify: fmt-check clippy-no-deps check test coverage-ci

verify-release: fmt-check clippy-all-target check test

all: verify build

coverage:
	rustup run $(TOOLCHAIN) cargo llvm-cov --bins --tests --workspace --open --ignore-filename-regex '(^.*/rustc-.*|^.*/lib/rustlib/.*)'

coverage-ci:
	rustup run $(TOOLCHAIN) cargo llvm-cov --bins --tests --workspace --ignore-filename-regex '(^.*/rustc-.*|^.*/lib/rustlib/.*)'
