# Surface v0.1.32 lists with media verification

Run from a checkout of `v0.1.32`. The base was
`28106f254bf686587bf239fb7da59560fcd164fa` (v0.1.31). Set `evidence_dir` to the
absolute path to this folder and `CARGO_TARGET_DIR` to the build directory in
/tmp for the checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-md3-list-media-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native offscreen Metal

Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI, and MCP returned identical PNGs.

The replay uses semantic actions. It checks the photo and poster in all
three styles, a controlled checkbox, the poster detail, RTL with large
text, and focus reveal in a short viewport. Its pixel probe checks the
crop and transparency. The Metal renderer tests check the two target
formats, clipped pixels, paint order and the texture cache separately.

[Native report](native/native-list-media-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Offscreen GPU tests](native/gpu-offscreen-tests.log) ·
[GPU texture cache](native/gpu-raster-cache.log)

[Light Baseline](native/media-baseline-light.png) ·
[Light Segmented](native/media-segmented-light.png) ·
[Dark Standard](native/media-standard-dark.png) ·
[Poster detail](native/media-poster-detail.png) ·
[Dark RTL at 200%](native/media-compact-dark-rtl.png) ·
[Short viewport](native/media-short-scroll.png)

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

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 11,802 checks. Material passed 190 steps and 11,554 checks.

M187–M191 use trusted pointer, touch and keyboard events. They check
the photo and poster sizes, the actual crop, alpha and corner pixels,
scroll clipping, legacy DOM export, named row actions, and controlled
checkboxes. The compact fixture uses 320 dp, RTL and 200% text.

[Browser report](web/browser-material-list-media.json) ·
[Browser commands](web/replay.md) ·
[Wide light](web/material-list-media-wide-light.png) ·
[Light Baseline](web/material-list-media-crop-baseline.png) ·
[Light Segmented](web/material-list-media-crop-segmented.png) ·
[Clipped artwork](web/material-list-media-clipped-portrait.png) ·
[Poster detail](web/material-list-media-detail.png) ·
[Dark RTL](web/material-list-media-compact-dark-rtl.png)

## Reference and remaining coverage

The Google Lists export has SHA-256
`0302dfe77765c4bd0aa25246b13bb8fddd561d3a10162256adc13a1eda226812`.
The token export has SHA-256
`f9e0966358c53123286e2c9f3539d2962ccef39349d85857552c02d629f0ed27`.

Google lists a 56 dp square thumbnail and two video poster profiles at
100 × 56 dp and 114 × 64 dp. Surface uses square Baseline media and 8 dp
Expressive corners. The Gallery chooses when to show the 80 dp photo
and the larger poster. An interactive Baseline poster uses a focus inset.
The sample artwork records its sources and hashes in the Gallery assets.

Physical VoiceOver speech, system high contrast, ripple, and timed hover
or press shapes remain open. The poster opens an app-owned detail view.

[Material 3 list guidelines](https://m3.material.io/components/lists/guidelines) ·
[Material 3 list specs](https://m3.material.io/components/lists/specs) ·
[Google token export](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.6c818a16475113bd.json) ·
[Gallery asset sources](https://github.com/faberline/surface/blob/v0.1.32/examples/material-gallery/assets/list-media/README.md) ·
[Surface #37](https://github.com/faberline/surface/issues/37)
