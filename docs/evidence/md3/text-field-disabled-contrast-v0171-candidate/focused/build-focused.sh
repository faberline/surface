#!/bin/sh
# This runs only with controller authorization.
set -eu
lane=$(CDPATH='' cd -- "$(dirname "$0")" && pwd -P)
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
test "$#" -eq 1
test "$1" = --build-scoped
(cd "$lane" && shasum -a 256 --status -c launcher.sha256)
test ! -e "$lane/pins"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${lane##*/}-source"
export CARGO_HOME=/tmp/surface-md3-cargo CARGO_BUILD_JOBS=2 CARGO_NET_OFFLINE=true
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 RUST_TEST_THREADS=1
export DEVELOPER_DIR=/Library/Developer/CommandLineTools SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
"$py" -E -B "$lane/prepare-focused.py" recheck-source
cd "$lane/source"
cap cargo build --offline --locked -p surface-e2e
"$py" -E -B "$lane/prepare-focused.py" recheck-source
mkdir "$lane/pins"
ln -s "$CARGO_TARGET_DIR/debug/surface-e2e" "$lane/pins/surface-e2e"
test -x "$lane/pins/surface-e2e"
shasum -a 256 "$lane/pins/surface-e2e" | awk '{print $1}' > "$lane/runner.sha256"
shasum -a 256 "$lane/focused-source.sha256" | awk '{print $1}' > "$lane/pins/focused-source-manifest.sha256"
