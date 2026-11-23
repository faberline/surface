# Surface v0.1.33 list corner motion verification

Run from a checkout of `v0.1.33`. The base was
`d110e9e7b80c0c4281f28443c9a07e0781decc71` (v0.1.32). Set `evidence_dir` to this
folder and set `CARGO_TARGET_DIR` to the checkout's build directory in /tmp.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-md3-list-corners-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native offscreen Metal

Native headless Metal reproduced 7 PNGs in two runs with zero visible windows. Page.screenshot, CLI, and MCP returned identical PNGs.

The native headless host applies Reduce Motion. The replay checks the
painted corner endpoints for the three styles, focus visibility in a
short viewport, stable selection and the focused row with media.

[Native report](native/native-list-corners-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Matching v0.1.32 endpoints](native/endpoint-baseline-comparison.json) ·
[Offscreen GPU tests](native/gpu-offscreen-tests.log) ·
[GPU texture cache](native/gpu-raster-cache.log)

[Segmented rest](native/list-corners-segmented-light-rest.png) ·
[Segmented focus](native/list-corners-segmented-light-focus.png) ·
[Dark Standard](native/list-corners-standard-dark-focus.png) ·
[Baseline](native/list-corners-baseline-light.png) ·
[Dark RTL at 200%](native/list-corners-compact-dark-rtl.png) ·
[Short viewport](native/list-corners-short-scroll.png) ·
[Focused media](native/list-corners-media-focus.png)

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

## Headless Chrome WebGPU

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 12,383 checks. Material passed 194 steps and 12,135 checks.

M192–M195 sample the GPU paint as real mouse, touch and keyboard events
change the state. They check motion between the existing token endpoints,
reversal, stable hit areas, one command per activation, focus paint and
Reduce Motion. The dark RTL fixture uses 320 dp width and 200% text.

[Browser report](web/browser-material-list-corners.json) ·
[Browser commands](web/replay.md) ·
[Hover midpoint](web/material-list-corners-hover-midpoint.png) ·
[Pressed midpoint](web/material-list-corners-pressed-midpoint.png) ·
[Focus midpoint](web/material-list-corners-focus-midpoint.png) ·
[Reduce Motion](web/material-list-corners-reduced.png) ·
[Compact dark RTL](web/material-list-corners-compact-dark-rtl-media.png)

The same focused test detected the old jump on the v0.1.32 Gallery at
frozen time zero. The first hover frame moved the edge from one to ten
device pixels. [Negative control](web/negative-control.json) ·
[Expected failure log](web/negative-control.log)

## Reference and remaining coverage

The Google Lists export SHA-256 is
`0302dfe77765c4bd0aa25246b13bb8fddd561d3a10162256adc13a1eda226812`.
The token export SHA-256 is
`f9e0966358c53123286e2c9f3539d2962ccef39349d85857552c02d629f0ed27`.
The unselected hover token points to the 12 dp medium shape. Focus,
press and selected state point to the 16 dp large shape. Surface uses
200 ms and its standard easing for this change.

List ripple, live VoiceOver speech and system high contrast remain open.

[Material 3 list specs](https://m3.material.io/components/lists/specs) ·
[Google token export](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.6c818a16475113bd.json) ·
[Surface #37](https://github.com/faberline/surface/issues/37)
