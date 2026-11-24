#!/bin/bash
set -euo pipefail
root=$(cd "$(dirname "$0")" && pwd -P)
stage=$1
manifest=$2
expected=$3
test ! -e "$root/source"
test "$(shasum -a 256 "$manifest" | cut -d ' ' -f 1)" = "$expected"
cp "$manifest" "$root/source.sha256"
(cd "$stage" && shasum -a 256 -c "$root/source.sha256") > "$root/incoming-source.log"
cp -cR "$stage" "$root/source"
cd "$root/source"
shasum -a 256 -c "$root/source.sha256" > "$root/build-source-before.log"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export CARGO_BUILD_JOBS=2 CARGO_HOME="${SURFACE_CARGO_HOME:-/tmp/surface-md3-cargo}" CARGO_NET_OFFLINE=true CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-$(basename "$root")"
export CARGO_PROFILE_DEV_DEBUG=0 SURFACE_WASM_OPT="${SURFACE_WASM_OPT:-/opt/homebrew/bin/wasm-opt}"
cap cargo xtask web build material-gallery --release --no-compress --out "$root/web/material-gallery" > "$root/gallery-build.log" 2>&1
shasum -a 256 -c "$root/source.sha256" > "$root/build-source-after.log"
cmp "$root/build-source-before.log" "$root/build-source-after.log"
(cd "$root" && shasum -a 256 web/material-gallery/* > gallery-web.sha256)
