# Surface v0.1.30 expandable Lists verification

Run from a checkout of `v0.1.30`. The base was
`408c58d944758f53a458b0c0433691076b1ef58e` (v0.1.29). Set `evidence_dir` to the
absolute path to this folder and `CARGO_TARGET_DIR` to the /tmp build directory
for the checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-md3-list-expansion-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native offscreen Metal

Native headless Metal reproduced 7 PNGs in two runs with zero visible windows. Page.screenshot, CLI, and MCP returned identical PNGs.

The replay covers three styles, named header and child actions, disabled child
skipping, focus return on Escape, and large RTL text in two short scroll areas.
The offscreen Gallery host uses Reduce Motion. The browser tests below sample
intermediate animation frames.

[Native report](native/native-list-expansion-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Offscreen GPU tests](native/gpu-offscreen-tests.log)

[Closed](native/expansion-segmented-light-closed.png) ·
[Open](native/expansion-segmented-light-open.png) ·
[After Escape](native/expansion-segmented-light-escape.png) ·
[Dark Standard](native/expansion-standard-dark.png) ·
[Light Baseline](native/expansion-baseline-light.png) ·
[Dark RTL at 200%](native/expansion-compact-dark-rtl.png) ·
[Short viewport](native/expansion-short-outer-scroll.png)

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

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 11,050 checks. Material passed 180 steps and 10,802 checks.

M178–M181 drive actual browser keyboard, mouse, wheel, and touch events.
They measure inline movement and reversal, check inactive child paint,
restore child focus to the header, and verify large RTL text at 320 dp.

[Browser report](web/browser-material-list-expansion.json) ·
[Browser commands](web/replay.md) ·
[Opening frame](web/material-list-expansion-opening-midpoint-light.png) ·
[Closing frame](web/material-list-expansion-closing-midpoint-light.png) ·
[Compact dark RTL](web/material-list-expansion-compact-dark-rtl-open.png)

## Reference and remaining coverage

The Google Lists guidelines export has SHA-256
`0302dfe77765c4bd0aa25246b13bb8fddd561d3a10162256adc13a1eda226812`.
The `md.comp.list.expand` token set is revision `d34f`. The token export has
SHA-256 `f9e0966358c53123286e2c9f3539d2962ccef39349d85857552c02d629f0ed27`.
Surface uses its Standard Default Spatial spring for this animation.

Physical VoiceOver speech, system high contrast, reorder controls, media
layouts, ripple, and timed hover or press shapes remain open.

[Material 3 list guidelines](https://m3.material.io/components/lists/guidelines) ·
[Material 3 list specs](https://m3.material.io/components/lists/specs) ·
[Google token export](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.6c818a16475113bd.json) ·
[Surface #37](https://github.com/faberline/surface/issues/37)
