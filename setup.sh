#!/usr/bin/env bash
set -euo pipefail

echo "rurl setup: start"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is not available. Install Rust toolchain first."
  exit 1
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is not available. Install rustup to manage components."
  exit 1
fi

toolchain="$(awk -F'"' '/^channel/ {print $2}' rust-toolchain.toml)"
if [ -z "${toolchain}" ]; then
  echo "toolchain channel not found in rust-toolchain.toml"
  exit 1
fi

echo "Using toolchain ${toolchain} from rust-toolchain.toml"
if ! rustup toolchain list | grep -Eq "^${toolchain}"; then
  echo "Installing Rust toolchain ${toolchain}..."
  rustup toolchain install "${toolchain}"
fi

echo "Installing llvm-tools-preview for toolchain ${toolchain}..."
rustup component add llvm-tools-preview --toolchain "${toolchain}"

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "Installing cargo-llvm-cov..."
  cargo install cargo-llvm-cov
else
  echo "cargo-llvm-cov already installed."
fi

echo "rurl setup: done"
