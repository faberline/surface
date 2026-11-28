#!/bin/sh
set -eu
lane=/tmp/surface-md3-v0176-current-replay.qSPAab9k
candidate=/tmp/surface-md3-v0176-forward.pjxmpm8l
source=$candidate/strict-replay
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT="$(xcrun --sdk macosx --show-sdk-path)"
export CARGO_HOME=/tmp/surface-md3-recovery-cargo.n0G16w5v
export CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-surface-md3-v0176-forward.pjxmpm8l-strict-replay
unset DISPLAY WAYLAND_DISPLAY WAYLAND_SOCKET
cd "$source"
sh "$lane/build.sh" --check-source
shasum -a 256 -c "$candidate/source.sha256" > "$candidate/workspace-manifest-before.log"
cap sh scripts/check.sh > "$candidate/workspace-current.log" 2>&1
shasum -a 256 -c "$candidate/source.sha256" > "$candidate/workspace-manifest-after.log"
cmp "$candidate/workspace-manifest-before.log" "$candidate/workspace-manifest-after.log"
printf '%s\n' b61630995f470772785605a944e3515a45b6d553 > "$candidate/workspace-current-passed"
tail -30 "$candidate/workspace-current.log"
