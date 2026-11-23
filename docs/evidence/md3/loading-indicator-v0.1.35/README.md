# Surface v0.1.35 Expressive loading indicator verification

Run from a checkout of `v0.1.35`. The base was
`cc5dfc81419d6f5702fbd9764bf0bb311fd5bc4f` (v0.1.34).
Set `evidence_dir` to this folder and `CARGO_TARGET_DIR` to the
build directory in /tmp.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-md3-loading-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native headless Gallery

Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.

The headless Gallery applies Reduce Motion and captures the fixed first
pose. Replays check both variants, three sizes, dark RTL at 200% text and
keyboard focus. The Metal test drives motion with an exact clock.

[Native report](native/native-loading-indicator-report.json) ·
[Replay comparison](native/native-repeat.json)

```sh
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
python3 -B "$evidence_dir/native/replay.py" --workspace "$PWD" \
    --source-manifest "$evidence_dir/native/native-source.sha256" \
    --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native"
mkdir -p "$scratch_dir/gpu"
SURFACE_GPU_EVIDENCE="$scratch_dir/gpu/gpu-offscreen.png" \
    cap cargo test --locked -p surface-wgpu --test it gpu_offscreen -- --ignored --nocapture
cap cargo test --locked -p surface-wgpu --lib gpu_offscreen_raster_cache -- --ignored --nocapture
```

[Metal test log](native/gpu-offscreen-tests.log) ·
[First pose](native/gpu-loading-light-first.png) ·
[Intermediate pose](native/gpu-loading-light-middle.png) ·
[Dark at 240 dp](native/gpu-loading-dark-large.png)

## Headless Chrome WebGPU

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 13,094 checks. Material passed 203 steps and 12,846 checks.

M201–M204 check paint and the real Chrome accessibility tree. They
measure seven distinct outlines, intermediate paint, the spring peak,
both timeline wraps, contrast and scaling. They also check frame
requests and focus while stopping, restarting and changing Reduce Motion.

[Browser report](web/browser-material-loading.json) ·
[Browser commands](web/replay.md) ·
[First pose](web/material-loading-sequence-light.png) ·
[Intermediate pose](web/material-loading-morph-midpoint.png) ·
[Dark RTL at 200%](web/material-loading-compact-dark-rtl.png) ·
[Reduce Motion](web/material-loading-reduced.png)

The pre-fix candidate changed Chrome accessibility IDs as the shape
changed. M202 reproduces this failure. The released animation updates
the contour in the display copy and keeps the Runtime semantic
generation. The same 13 clocked PNGs match byte for byte.
[Pre-fix report](web/pre-fix-ax.json) · [Run log](web/pre-fix-ax.log) ·
[PNG parity](web/pre-fix-raster-parity.json)

## Reference and remaining coverage

The 38:48 active-to-outline size ratio and the variant color roles follow
Google's published specs. The dated page export hash is
`79b51c48c7c7b58aab5ad993baadc6386535eeb51bd848165313d483a89da31b`.
The token export hash is
`8b8227b07acccd46359827f3811b4c138aca3c6818413730f1f4bebae8b3349d`.

The motion uses the pinned AndroidX sequence and spring. Surface eases
the spring tail from 297 to 450 ms to keep rotation continuous. Reduce
Motion uses the first pose. Pull-to-refresh, the AndroidX determinate
overload, custom palettes, live screen-reader speech and system high
contrast remain open.

[Material 3 specs](https://m3.material.io/components/loading-indicator/specs) ·
[Published page export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/eb6cf431-55ab-4d41-b3c5-cf4da1bae66f.json) ·
[Published token export](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.68895be451a51c31.json) ·
[Derived contour verification](contour-generation.json) ·
[Surface #22](https://github.com/faberline/surface/issues/22)
