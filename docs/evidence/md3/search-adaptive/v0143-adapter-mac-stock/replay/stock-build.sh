#!/bin/bash
set -euo pipefail

scratch=$(cd "$(dirname "$0")" && pwd)
source=$1
incoming=$2
stage=$3
prior=$4
case "$stage" in "$scratch"/stock.*) ;; *) exit 2 ;; esac
test -d "$stage"
test ! -e "$stage/stock-stage.json"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:/Users/chrischeng/.pyenv/versions/3.12.11/bin:$PATH"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export CARGO_BUILD_JOBS=2 CARGO_HOME=/tmp/surface-md3-cargo CARGO_NET_OFFLINE=true
export PYTHONDONTWRITEBYTECODE=1
python3 -B "$scratch/stock-inputs.py" prepare "$stage" "$source" "$incoming"
python3 -B "$scratch/verify-payload-reuse.py" preflight "$prior" "$stage"
(cd "$prior" && shasum -a 256 -c sealed-inputs.sha256) > "$stage/prior-stage.log"
(cd "$prior/bundle" && shasum -a 256 -c "$prior/browser-binary-sha256.txt") > "$stage/prior-bundle.log"
cp "$prior/incoming-source.sha256" "$stage/prior-incoming-source.sha256"
cp "$prior/browser-binary-sha256.txt" "$stage/prior-binary-sha256.txt"
cp "$scratch/adapter-check.patch" "$stage/adapter-check.patch"
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-$(basename "$stage")"
test ! -e "$CARGO_TARGET_DIR"
printf '%s\n' "$stage"
cd "$source"

{
    rustc --version
    cargo --version
    cap cargo fmt -p surface-e2e --check
    cap cargo clippy --locked -p surface-e2e --bin surface-e2e -- -D warnings
    cap cargo test --locked -p surface-e2e --bin surface-e2e
    cap cargo build --locked -p surface-e2e --bin surface-e2e
} 2>&1 | tee "$stage/build.log"

python3 -B "$scratch/stock-inputs.py" verify "$stage" "$stage/built-input-sha256.txt"
bundle="$stage/bundle"
mkdir -p "$bundle/debug" "$bundle/web/release" "$bundle/web/devtools"
cp -p "$CARGO_TARGET_DIR/debug/surface-e2e" "$bundle/debug/"
cp -p "$prior"/bundle/debug/{surface-material-gallery,surface-devtools,jet-bridge} "$bundle/debug/"
cp -Rp "$prior"/bundle/web/release/{material-gallery,showcase} "$bundle/web/release/"
cp -Rp "$prior/bundle/web/devtools/material-gallery" "$bundle/web/devtools/"
(
    cd "$bundle"
    shasum -a 256 debug/{surface-e2e,surface-material-gallery,surface-devtools,jet-bridge} \
        web/release/{material-gallery,showcase}/{*.wasm,*.js,index.html,surface-web.json} \
        web/devtools/material-gallery/{*.wasm,*.js,index.html,surface-web.json}
) > "$stage/browser-binary-sha256.txt"
python3 -B "$scratch/verify-payload-reuse.py" seal "$prior" "$stage"
(cd "$bundle" && shasum -a 256 -c "$stage/browser-binary-sha256.txt")
(
    cd "$scratch"
    shasum -a 256 stock-build.sh stock-run.sh stock-inputs.py verify-stock.py verify-payload-reuse.py verify_*_report.py
) > "$stage/harness.sha256"
(
    cd "$stage"
    shasum -a 256 stock-stage.json incoming-source.sha256 browser-source.sha256 browser-binary-sha256.txt harness.sha256 \
        payload-reuse.json prior-incoming-source.sha256 prior-binary-sha256.txt adapter-check.patch
) > "$stage/sealed-inputs.sha256"
printf 'Stock bundle sealed at %s\n' "$bundle"
