#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
mkdir -p artifacts
export SURFACE_GPU_EVIDENCE="$PWD/artifacts/gpu-offscreen.png"
cap cargo test --locked -p surface-wgpu --test it gpu_offscreen_paints_clips_rounding_text_and_order -- --ignored --exact --nocapture
