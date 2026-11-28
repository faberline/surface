#!/bin/sh
set -eu
umask 077
lane=$(CDPATH='' cd -- "$(dirname "$0")" && pwd -P)
candidate="$lane"
source=/tmp/surface-md3-choice-menu-v0176-corrected.2R544r7F/source
target=/tmp/cargo-target/surface-surface-md3-choice-menu-v0176-corrected.2R544r7F-source
cache=/tmp/surface-md3-recovery-cargo.n0G16w5v
py=${SURFACE_REPLAY_PYTHON:-python3}

case "${1:-}" in
    --check-source)
        test "$#" -eq 1
        exec "$py" -E -B "$lane/pins.py" source "$candidate"
        ;;
    --verify)
        test "$#" -eq 2
        exec "$py" -E -B "$lane/pins.py" verify "$candidate" "$2"
        ;;
    --build)
        test "$#" -eq 2
        ;;
    *)
        printf '%s\n' "Usage: sh $0 --check-source | --build OUT | --verify OUT" >&2
        exit 2
        ;;
esac

"$py" -E -B "$lane/pins.py" source "$candidate"
build_out=$("$py" -E -B "$lane/pins.py" reserve "$candidate" "$2")
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export CARGO_HOME="$cache"
export CARGO_TARGET_DIR="$target"
export CARGO_BUILD_JOBS=2
unset DISPLAY WAYLAND_DISPLAY WAYLAND_SOCKET SURFACE_CHROME_ARGS SURFACE_CHROME
test -d "$CARGO_HOME"
mkdir "$build_out/native" "$build_out/runners" "$build_out/web"
cd "$source"

{
    sw_vers
    uname -m
    cap rustc --version
    cap cargo --version
    cap wasm-bindgen --version
    cap wasm-opt --version
} > "$build_out/toolchain.txt" 2>&1
printf '%s\n' "Build output: $build_out"

if ! cap cargo xtask web build material-gallery --release --no-compress \
        --out "$build_out/web/material-gallery" > "$build_out/web-build.log" 2>&1; then
    cat "$build_out/web-build.log" >&2
    exit 1
fi
if ! cap cargo build --locked -p surface-e2e --bin surface-e2e \
        > "$build_out/runner-build.log" 2>&1; then
    cat "$build_out/runner-build.log" >&2
    exit 1
fi
if ! cap cargo build --locked --release -p surface-material-gallery \
        --features surface-material-gallery/devtools -p surface-devtools \
        > "$build_out/native-build.log" 2>&1; then
    cat "$build_out/native-build.log" >&2
    exit 1
fi

install -m 0755 "$target/debug/surface-e2e" "$build_out/runners/surface-e2e"
install -m 0755 "$target/release/surface-material-gallery" "$build_out/native/surface-material-gallery"
install -m 0755 "$target/release/surface-devtools" "$build_out/native/surface-devtools"
"$py" -E -B "$lane/pins.py" seal "$candidate" "$build_out"
"$py" -E -B "$lane/pins.py" verify "$candidate" "$build_out"
