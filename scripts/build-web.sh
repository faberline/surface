#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
cap cargo build --locked --target wasm32-unknown-unknown -p surface-showcase --lib
wasm-bindgen --target web --out-dir examples/showcase/web/pkg target/wasm32-unknown-unknown/debug/surface_showcase.wasm
