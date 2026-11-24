#!/bin/bash
set -euo pipefail
root=$(cd "$(dirname "$0")" && pwd -P)
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export CARGO_BUILD_JOBS=2 CARGO_HOME="${SURFACE_CARGO_HOME:-/tmp/surface-md3-cargo}" CARGO_NET_OFFLINE=true CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-$(basename "$root")"
export CARGO_PROFILE_DEV_DEBUG=0
cd "$root/runner-stock"
cap cargo fmt -p surface-e2e --check > "$root/runner-fmt.log" 2>&1
cd "$root/focused-source"
shasum -a 256 -c "$root/focused-source.sha256" > "$root/runner-source-before.log"
cap cargo build --locked -p surface-e2e --bin surface-e2e > "$root/runner-build.log" 2>&1
shasum -a 256 -c "$root/focused-source.sha256" > "$root/runner-source-after.log"
cmp "$root/runner-source-before.log" "$root/runner-source-after.log"
mkdir "$root/pins"
cp "$CARGO_TARGET_DIR/debug/surface-e2e" "$root/pins/"
(cd "$root" && shasum -a 256 -c gallery-web.sha256 && shasum -a 256 web/material-gallery/* pins/surface-e2e > runtime.sha256)
