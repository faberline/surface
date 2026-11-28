#!/bin/sh
set -eu
umask 077
lane=/tmp/surface-md3-v0176-current-replay.qSPAab9k
source=/tmp/surface-md3-v0176-forward.pjxmpm8l/strict-replay
old_lock=/tmp/surface-md3-choice-menu-v0176-diagnostic.sUVCw13u/appkit/Cargo.lock
target=/tmp/cargo-target/surface-surface-md3-v0176-forward.pjxmpm8l-strict-replay
test "$#" -eq 1
case "$1" in /tmp/*) ;; *) exit 2 ;; esac
test "$(shasum -a 256 "$old_lock" | cut -d ' ' -f 1)" = 1f4fd82e49595b9727076ed73b3490571c7b470d274e02983803eaadd5bc4d12
sh "$lane/build.sh" --check-source
mkdir "$1"
out=$(CDPATH='' cd -- "$1" && pwd -P)
cp -R "$source/third-party/accesskit-macos" "$out/accesskit-macos"
cp "$old_lock" "$out/accesskit-macos/Cargo.lock"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export CARGO_TARGET_DIR="$target" CARGO_BUILD_JOBS=2
unset DISPLAY WAYLAND_DISPLAY WAYLAND_SOCKET
cap cargo test --locked --manifest-path "$out/accesskit-macos/Cargo.toml" --test menu_item_mark_char > "$out/appkit.log" 2>&1
sh "$lane/build.sh" --check-source
printf '%s\n' 'Detached NSView AppKit selector test exited successfully.' > "$out/APPKIT-PASSED"
shasum -a 256 "$out/accesskit-macos/Cargo.lock" "$out/appkit.log"
