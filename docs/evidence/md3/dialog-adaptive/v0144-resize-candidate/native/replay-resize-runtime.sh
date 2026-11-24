#!/bin/sh
set -eu

batch=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export CARGO_HOME=${SURFACE_QA_CARGO_HOME:-/tmp/surface-md3-cargo}
export CARGO_TARGET_DIR=${SURFACE_QA_TARGET_DIR:-/tmp/cargo-target/surface-${batch##*/}}
export CARGO_BUILD_JOBS=2 CARGO_NET_OFFLINE=true CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 RUST_TEST_THREADS=1
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
cd "$batch/source"
cap cargo test --locked -p surface-material-gallery --features devtools --lib \
    dialog_demo::tests::resting_fab_remeasures_after_resize_without_an_input_event -- --exact
