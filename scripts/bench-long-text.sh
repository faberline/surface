#!/bin/sh
# Long, growing CJK text benchmark (issue #107). Needs a real GPU adapter.
# Not part of scripts/check.sh or CI. It has no threshold.
set -eu
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
cap cargo run --locked --release -p surface-wgpu --example long_text_bench
