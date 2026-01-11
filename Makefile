.PHONY: fmt fmt-check clippy-all-target clippy-no-deps check test build lint verify verify-release coverage coverage-ci dist dist-clean

TOOLCHAIN := $(shell awk -F'"' '/^channel/ {print $$2}' rust-toolchain.toml)
VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
HOST_TARGET := $(shell rustc -Vv | awk '/host/ {print $$2}')
EXE_SUFFIX := $(if $(findstring windows,$(HOST_TARGET)),.exe,)
DIST_DIR := dist
PACKAGE_BASENAME := $(DIST_DIR)/rurl-$(VERSION)-$(HOST_TARGET)
BIN_PATH := target/release/rurl$(EXE_SUFFIX)

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

dist:
	cargo build --release
	rm -rf $(PACKAGE_BASENAME) $(PACKAGE_BASENAME).tar.gz
	mkdir -p $(PACKAGE_BASENAME)
	cp $(BIN_PATH) $(PACKAGE_BASENAME)/
	cp README.md LICENSE $(PACKAGE_BASENAME)/
	cp docs/LOCAL_DEVELOPMENT.md $(PACKAGE_BASENAME)/
	tar -C $(DIST_DIR) -czf $(PACKAGE_BASENAME).tar.gz $(notdir $(PACKAGE_BASENAME))

dist-clean:
	rm -rf $(DIST_DIR)
