#!/bin/sh
set -eu
umask 077
lane=$(CDPATH='' cd -- "$(dirname "$0")" && pwd -P)
source=$lane/surface
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT="$(xcrun --sdk macosx --show-sdk-path)"
export CARGO_HOME=/tmp/surface-md3-recovery-cargo.n0G16w5v
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-surface-md3-v0178-menu-system-colors.hlLwvQTS-surface
export CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
unset DISPLAY WAYLAND_DISPLAY WAYLAND_SOCKET SURFACE_CHROME SURFACE_CHROME_ARGS
mode=$1
expected_tree=$2
expected_manifest=$3
verify_source() {
    test "$(git -C "$source" rev-parse 'HEAD^{tree}')" = "$expected_tree"
    test -z "$(git -C "$source" --no-optional-locks status --porcelain)"
    test "$(shasum -a 256 "$lane/source.sha256" | cut -d ' ' -f 1)" = "$expected_manifest"
    (cd "$source" && shasum -a 256 -c "$lane/source.sha256")
}
if test "$mode" = --check-source; then
    test "$#" -eq 3
    verify_source
    exit 0
fi
test "$mode" = --build
test "$#" -eq 4
case "$4" in /tmp/*) ;; *) exit 2 ;; esac
mkdir "$4"
out=$(CDPATH='' cd -- "$4" && pwd -P)
verify_source > "$out/source-before.log"
mkdir -p "$out/native" "$out/stock/debug" "$out/stock/web/release" "$out/stock/web/devtools"
cd "$source"
{ sw_vers; uname -m; cap rustc --version; cap cargo --version; cap wasm-bindgen --version; cap wasm-opt --version; } > "$out/toolchain.txt" 2>&1
{
    cap cargo xtask web build showcase --release --no-compress --out "$out/stock/web/release/showcase"
    cap cargo xtask web build material-gallery --release --no-compress --out "$out/stock/web/release/material-gallery"
    cap cargo xtask web build material-gallery --release --features devtools --no-compress --out "$out/stock/web/devtools/material-gallery"
} > "$out/web-build.log" 2>&1
cap cargo build --locked -p surface-material-gallery --features surface-material-gallery/devtools -p surface-devtools > "$out/native-build.log" 2>&1
{
    cap cargo build --locked -p surface-e2e --bin surface-e2e
    cap cargo build --locked -p jet-bridge --bin jet-bridge
} > "$out/stock-build.log" 2>&1
for name in surface-material-gallery surface-devtools; do install -m 0755 "$CARGO_TARGET_DIR/debug/$name" "$out/native/$name"; done
for name in surface-e2e jet-bridge; do install -m 0755 "$CARGO_TARGET_DIR/debug/$name" "$out/stock/debug/$name"; done
verify_source > "$out/source-after.log"
cmp "$out/source-before.log" "$out/source-after.log"
(cd "$out" && rg --files --hidden --no-ignore native stock | LC_ALL=C sort | xargs shasum -a 256 > SHA256SUMS)
(cd "$out/native" && shasum -a 256 surface-material-gallery surface-devtools > SHA256SUMS)
printf '%s\n' "$expected_tree" "$expected_manifest" > "$out/BUILD-COMPLETE"
printf 'Build output: %s\n' "$out"
