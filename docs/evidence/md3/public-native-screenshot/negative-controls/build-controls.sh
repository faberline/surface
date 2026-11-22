#!/bin/sh
set -eu
proof="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
product="$1"
target="/tmp/cargo-target/$(basename "$proof" | tr . -)"
export CARGO_HOME=/tmp/surface-md3-cargo CARGO_TARGET_DIR="$target"
export CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0

cd "$product"
shasum -a 256 Cargo.lock crates/surface-platform/src/screenshot.rs \
  e2e/src/devtools_native.rs e2e/src/main.rs > "$proof/product-source.sha256"
cp -cR "$product" "$proof/repo"
cd "$proof/repo"
shasum -a 256 -c "$proof/product-source.sha256" > "$proof/copy-check.log"
python3 "$proof/inject.py" "$proof/repo" > "$proof/inject.log"
shasum -a 256 e2e/src/devtools_native.rs e2e/src/main.rs > "$proof/runner-source.sha256"

mkdir -p "$proof/bin"
printf 'CARGO_TARGET_DIR=%s\n' "$target" > "$proof/build-command.txt"
printf '%s\n' 'cap cargo build --locked -p surface-material-gallery -p surface-devtools -p surface-e2e --features surface-material-gallery/devtools --bin surface-material-gallery --bin surface-devtools --bin surface-e2e' >> "$proof/build-command.txt"
cap cargo build --locked -p surface-material-gallery -p surface-devtools -p surface-e2e \
  --features surface-material-gallery/devtools --bin surface-material-gallery \
  --bin surface-devtools --bin surface-e2e --message-format=json \
  > "$proof/build.jsonl" 2> "$proof/build.log"
for binary in surface-material-gallery surface-devtools surface-e2e; do
    cp "$target/debug/$binary" "$proof/bin/$binary"
done
shasum -a 256 "$proof/bin/"* > "$proof/binaries.sha256"
shasum -a 256 -c "$proof/runner-source.sha256" > "$proof/runner-source-check.log"
cd "$product"
shasum -a 256 -c "$proof/product-source.sha256" > "$proof/product-check-after-build.log"
