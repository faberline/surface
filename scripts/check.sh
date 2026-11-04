#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
cap cargo fmt --all --check
cap cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cap cargo test --locked --workspace --all-features
cap cargo check --locked --target wasm32-unknown-unknown -p surface-showcase -p surface-material-gallery --lib
