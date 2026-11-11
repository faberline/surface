#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
cap cargo xtask provenance
cap cargo fmt --all --check
cap cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cap cargo test --locked --workspace --all-features
cap cargo check --locked --target wasm32-unknown-unknown -p surface-showcase -p surface-material-gallery --all-features --lib
# Jet builds ui-runtime for wasm32 with and without `debug`. The consumer
# contract test pins the surface and ui-runtime API that Jet and Core use.
cap cargo check --locked --target wasm32-unknown-unknown -p ui-runtime --all-features --lib --test consumer_contract
cap cargo check --locked --target wasm32-unknown-unknown -p ui-runtime --no-default-features --lib --test consumer_contract
