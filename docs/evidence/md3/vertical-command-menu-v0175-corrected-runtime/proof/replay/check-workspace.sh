#!/bin/sh
set -eu
umask 077
lane=$(CDPATH='' cd -- "$(dirname "$0")" && pwd -P)
source=/tmp/surface-md3-disabled-menu-click.qfrcXZ1r/source
py=${SURFACE_REPLAY_PYTHON:-python3}
test "$#" -eq 1

"$py" -E -B "$lane/pins.py" source "$lane"
out=$("$py" -E -B "$lane/pins.py" reserve "$lane" "$1")
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export CARGO_HOME=/tmp/surface-md3-recovery-cargo.n0G16w5v
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-surface-md3-disabled-menu-click.qfrcXZ1r-source
export CARGO_BUILD_JOBS=2
unset DISPLAY WAYLAND_DISPLAY WAYLAND_SOCKET SURFACE_CHROME_ARGS SURFACE_CHROME
test -d "$CARGO_HOME"
if ! (cd "$source" && sh scripts/check.sh) > "$out/check.log" 2>&1; then
    tail -80 "$out/check.log" >&2
    exit 1
fi
"$py" -E -B "$lane/pins.py" source "$lane"
"$py" -E -B "$lane/workspace-receipt.py" "$out"
cp "$lane/source.tree" "$out/tree"
