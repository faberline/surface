#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
cap cargo build --locked --target wasm32-unknown-unknown -p surface-material-gallery --lib
wasm-bindgen --target web --out-dir examples/material-gallery/web/pkg target/wasm32-unknown-unknown/debug/surface_material_gallery.wasm
