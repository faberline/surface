#!/bin/sh
set -eu
umask 077
lane=$(CDPATH='' cd -- "$(dirname "$0")" && pwd -P)
source=/tmp/surface-md3-label-color-v0168-corrected.6WxZjn4h/surface
target=/tmp/cargo-target/surface-surface-md3-label-color-v0168-corrected.6WxZjn4h-surface
py=${SURFACE_REPLAY_PYTHON:-python3}

case "${1:-}" in
    --check-source) test "$#" -eq 1; exec "$py" -E -B "$lane/pins.py" source ;;
    --verify) test "$#" -eq 2; exec "$py" -E -B "$lane/pins.py" verify "$2" ;;
    --build) test "$#" -eq 2 ;;
    *) exit 2 ;;
esac

"$py" -E -B "$lane/pins.py" source
out=$("$py" -E -B "$lane/pins.py" reserve "$2")
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export CARGO_HOME=/tmp/surface-md3-recovery-cargo.n0G16w5v
export CARGO_TARGET_DIR="$target"
export CARGO_BUILD_JOBS=2
unset DISPLAY WAYLAND_DISPLAY WAYLAND_SOCKET SURFACE_CHROME SURFACE_CHROME_ARGS
test -d "$CARGO_HOME"
mkdir -p "$out/native" "$out/stock/debug" "$out/stock/web/release" "$out/stock/web/devtools"
cd "$source"
{
    sw_vers
    uname -m
    cap rustc --version
    cap cargo --version
    cap wasm-bindgen --version
    cap wasm-opt --version
} > "$out/toolchain.txt" 2>&1
printf '%s\n' "Build output: $out"

if ! (
    cap cargo xtask web build showcase --release --no-compress --out "$out/stock/web/release/showcase" &&
    cap cargo xtask web build material-gallery --release --no-compress --out "$out/stock/web/release/material-gallery" &&
    cap cargo xtask web build material-gallery --release --features devtools --no-compress \
        --out "$out/stock/web/devtools/material-gallery"
) > "$out/web-build.log" 2>&1; then
    tail -80 "$out/web-build.log" >&2
    exit 1
fi
if ! cap cargo build --locked -p surface-material-gallery --features surface-material-gallery/devtools \
        -p surface-devtools > "$out/native-build.log" 2>&1; then
    tail -80 "$out/native-build.log" >&2
    exit 1
fi
if ! (
    cap cargo build --locked -p surface-e2e --bin surface-e2e &&
    cap cargo build --locked -p jet-bridge --bin jet-bridge
) > "$out/stock-build.log" 2>&1; then
    tail -80 "$out/stock-build.log" >&2
    exit 1
fi
for name in surface-material-gallery surface-devtools; do
    install -m 0755 "$target/debug/$name" "$out/native/$name"
done
for name in surface-e2e jet-bridge; do
    install -m 0755 "$target/debug/$name" "$out/stock/debug/$name"
done
"$py" -E -B "$lane/pins.py" seal "$out"
"$py" -E -B "$lane/pins.py" verify "$out"
