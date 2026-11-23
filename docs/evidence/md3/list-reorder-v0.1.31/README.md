# Surface v0.1.31 reorderable Lists verification

Run from a checkout of `v0.1.31`. The base was
`e206353e491d241fb67455e74d60a6bb4dd21eb7` (v0.1.30). Set `evidence_dir` to the
absolute path to this folder and `CARGO_TARGET_DIR` to the build directory in
/tmp for the checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-md3-list-reorder-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native offscreen Metal

Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI, and MCP returned identical PNGs.

The replay uses semantic key and activation actions. It checks three list
styles, the fixed row, focus after a move, a separate primary command and
the short named scroller. The Web replay below checks physical dragging.

[Native report](native/native-list-reorder-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Offscreen GPU tests](native/gpu-offscreen-tests.log)

[Before](native/reorder-segmented-light-before.png) ·
[After](native/reorder-segmented-light-after.png) ·
[Dark Standard](native/reorder-standard-dark.png) ·
[Light Baseline](native/reorder-baseline-light.png) ·
[Dark RTL at 200%](native/reorder-compact-dark-rtl.png) ·
[Short viewport](native/reorder-short-scroll.png)

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
```

## Headless Chrome WebGPU

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 11,482 checks. Material passed 185 steps and 11,234 checks.

M182–M186 use trusted pointer, touch, and keyboard events. They check
move commands, the fixed barrier, active paint and focus, and drop after
edge scrolling without another pointer move. They cancel through Escape,
capture loss, touch cancellation and an external order change. The compact
fixture uses 320 dp, RTL, 200% text and Reduce Motion.

[Browser report](web/browser-material-list-reorder.json) ·
[Browser commands](web/replay.md) ·
[Expressive held](web/material-list-reorder-expressive-held.png) ·
[Baseline held](web/material-list-reorder-baseline-held.png) ·
[After edge scrolling](web/material-list-reorder-edge-release.png) ·
[Compact dark RTL](web/material-list-reorder-compact-dark-rtl.png)

## Reference and remaining coverage

The Google Lists export has SHA-256
`0302dfe77765c4bd0aa25246b13bb8fddd561d3a10162256adc13a1eda226812`.
The `md.comp.list.reorder` token set is revision `2320`. The token export
has SHA-256
`f9e0966358c53123286e2c9f3539d2962ccef39349d85857552c02d629f0ed27`.
Surface chooses the 8 dp destination marker and the edge scroll speed.

Physical VoiceOver speech, system high contrast, media layouts, ripple,
and timed hover or press shapes remain open.

[Material 3 list guidelines](https://m3.material.io/components/lists/guidelines) ·
[Material 3 list specs](https://m3.material.io/components/lists/specs) ·
[Google token export](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.6c818a16475113bd.json) ·
[WCAG dragging movements](https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements.html) ·
[Surface #37](https://github.com/faberline/surface/issues/37)
