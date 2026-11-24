#!/bin/bash
set -euo pipefail
root=$(cd "$(dirname "$0")" && pwd -P)
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export CARGO_BUILD_JOBS=2 CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-false}"
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-$(basename "$root")"
pin=$1
case "$pin" in "$root"/focused.*) ;; *) exit 2;; esac
cd "$root/focused-source"
cap cargo build --locked -p surface-e2e --bin surface-e2e 2>&1 | tee "$pin/build.log"
cp "$CARGO_TARGET_DIR/debug/surface-e2e" "$pin/surface-e2e"
shasum -a 256 e2e/src/{main,material,material_dialog,material_time_input,browser,check,keys,server,smoke}.rs > "$pin/runner-source.sha256"
(cd "$root" && shasum -a 256 web/material-gallery/* "$(basename "$pin")/surface-e2e") > "$pin/runtime.sha256"
