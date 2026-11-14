#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
mkdir -p artifacts
export SURFACE_GPU_EVIDENCE="$PWD/artifacts/gpu-offscreen.png"
# Runs every ignored GPU test: gpu_offscreen_paints_clips_rounding_text_and_order,
# gpu_offscreen_bottom_corners_are_not_swapped,
# gpu_offscreen_per_corner_shapes, gpu_offscreen_motion_animates_color_and_corners
# and gpu_offscreen_capture_reads_rgba_in_both_formats (Renderer::capture; writes
# gpu-capture.png next to the evidence file).
# gpu_offscreen_per_corner_shapes writes gpu-shapes.png and
# gpu_offscreen_motion_animates_color_and_corners writes
# gpu-motion-midpoint.png next to the evidence file.
cap cargo test --locked -p surface-wgpu --test it gpu_offscreen -- --ignored --nocapture
