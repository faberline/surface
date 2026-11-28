#!/bin/sh
set -eu
lane=/tmp/surface-md3-v0177-submenus.P7WLBuVn
source=$lane/surface
out=$1
case "$out" in "$lane"/*) ;; *) exit 2 ;; esac
mkdir "$out"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT="$(xcrun --sdk macosx --show-sdk-path)"
export CARGO_HOME=/tmp/surface-md3-recovery-cargo.n0G16w5v
export CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-surface-md3-v0177-submenus.P7WLBuVn-surface
unset DISPLAY WAYLAND_DISPLAY WAYLAND_SOCKET
cd "$source"
test "$(git show --no-patch --format='%T' HEAD)" = b61630995f470772785605a944e3515a45b6d553
cap cargo test --locked -p surface-app --all-features --test modal_submenu_support > "$out/runtime.log" 2>&1
cap cargo test --locked -p surface-material --all-features --test command_menu > "$out/material.log" 2>&1
cap cargo test --locked -p surface-platform --all-features --lib an_accesskit_menu_trigger_reports_popup_and_expansion > "$out/accesskit.log" 2>&1
rg -n 'test result:' "$out/runtime.log" "$out/material.log" "$out/accesskit.log"
