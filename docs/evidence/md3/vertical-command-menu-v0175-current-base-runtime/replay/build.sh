#!/bin/sh
set -eu
umask 077
lane=$(CDPATH='' cd -- "$(dirname "$0")" && pwd -P)
source=/tmp/surface-md3-v0175-current-base.rWr9qUiy/source
target=/tmp/cargo-target/surface-surface-md3-v0175-current-base.rWr9qUiy-source
py=${SURFACE_REPLAY_PYTHON:-python3}
"$py" -E -B "$lane/source-check.py"
case "${1:-}" in
  --check-source) test "$#" -eq 1; exit 0 ;;
  --build) test "$#" -eq 2 ;;
  *) printf '%s\n' 'Usage: sh build.sh --check-source | --build /tmp/new-output-directory' >&2; exit 2 ;;
esac
case "$2" in /tmp/*) ;; *) printf '%s\n' 'Output must be a new directory under /tmp.' >&2; exit 2 ;; esac
mkdir "$2"
out=$(CDPATH='' cd -- "$2" && pwd -P)
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export CARGO_TARGET_DIR="$target" CARGO_BUILD_JOBS=2
unset DISPLAY WAYLAND_DISPLAY WAYLAND_SOCKET SURFACE_CHROME SURFACE_CHROME_ARGS
mkdir -p "$out/native" "$out/stock/debug" "$out/stock/web/release" "$out/stock/web/devtools"
cd "$source"
{ sw_vers; uname -m; cap rustc --version; cap cargo --version; cap wasm-bindgen --version; cap wasm-opt --version; } > "$out/toolchain.txt" 2>&1
{ cap cargo xtask web build showcase --release --no-compress --out "$out/stock/web/release/showcase";
  cap cargo xtask web build material-gallery --release --no-compress --out "$out/stock/web/release/material-gallery";
  cap cargo xtask web build material-gallery --release --features devtools --no-compress --out "$out/stock/web/devtools/material-gallery"; } > "$out/web-build.log" 2>&1
cap cargo build --locked -p surface-material-gallery --features surface-material-gallery/devtools -p surface-devtools > "$out/native-build.log" 2>&1
{ cap cargo build --locked -p surface-e2e --bin surface-e2e; cap cargo build --locked -p jet-bridge --bin jet-bridge; } > "$out/stock-build.log" 2>&1
for name in surface-material-gallery surface-devtools; do install -m 0755 "$target/debug/$name" "$out/native/$name"; done
for name in surface-e2e jet-bridge; do install -m 0755 "$target/debug/$name" "$out/stock/debug/$name"; done
"$py" -E -B "$lane/source-check.py"
(cd "$out" && find native stock -type f -print | LC_ALL=C sort | xargs shasum -a 256 > SHA256SUMS)
printf '%s\n' 'Build completed. Runtime tests have not run.' > "$out/BUILD-COMPLETE"
printf 'Build output: %s\n' "$out"
