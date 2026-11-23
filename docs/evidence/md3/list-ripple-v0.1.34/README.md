# Surface v0.1.34 list ripple verification

Run from a checkout of `v0.1.34`. The base was
`289574bb438cd88fc10b62d915f8a93552554548` (v0.1.33). Set `evidence_dir` to
this folder and `CARGO_TARGET_DIR` to the build directory in /tmp.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-md3-list-ripple-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native headless Gallery

Native headless Metal reproduced 7 PNGs in two runs with zero visible windows. Page.screenshot, CLI, and MCP returned identical PNGs.

The Gallery headless host applies Reduce Motion. These captures check
the three list styles, short scrolling, narrow RTL at 200% text, focus
and media with that setting. The offscreen Metal test below drives the
Runtime with pointer input and an explicit clock to verify moving pixels.

[Native report](native/native-list-ripple-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Matching v0.1.33 endpoints](native/prior-endpoint-parity.json) ·
[Dark RTL at 200%](native/list-ripple-compact-dark-rtl.png) ·
[Short viewport](native/list-ripple-short-scroll.png) ·
[Media focus](native/list-ripple-media-focus.png)

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

The clocked test samples the photo, a far corner, the focus stroke,
a foreground marker and the region behind the clip. It verifies
growth, released fade and the static pressed state with Reduce Motion.

[Metal test log](native/gpu-offscreen-tests.log) ·
[Early](native/gpu-list-ripple-early.png) ·
[Full](native/gpu-list-ripple-full.png) ·
[Fade](native/gpu-list-ripple-fade.png) ·
[Reduce Motion](native/gpu-list-ripple-reduced.png)

## Headless Chrome WebGPU

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 12,641 checks. Material passed 199 steps and 12,393 checks.

M196–M200 use trusted mouse and touch events. They sample the press
origin, grow and fade the wave, handle quick and cancelled taps, check
the order of media, focus and text, and repeat in narrow dark RTL.
They also switch Reduce Motion during a press.

[Browser report](web/browser-material-list-ripple.json) ·
[Browser commands](web/replay.md) ·
[Off-center press](web/material-list-ripple-off-center-early.png) ·
[Expanded](web/material-list-ripple-bounded-expanded.png) ·
[Release fade](web/material-list-ripple-release-midpoint.png) ·
[Media focus](web/material-list-ripple-media-focus.png) ·
[Compact dark RTL](web/material-list-ripple-compact-dark-rtl.png) ·
[Reduce Motion](web/material-list-ripple-reduced-static.png)

The same focused test detects the immediate full-row tint on the v0.1.33
Gallery at time zero. [Negative control](web/negative-control.json) ·
[Expected failure log](web/negative-control.log)

## Reference and remaining coverage

Google shows a touch ripple on Material list rows and defines a 10%
pressed state layer. Surface uses the existing 225 ms expansion, 75 ms
fade-in and 150 ms release fade policy. Ripple coverage for expandable
and reorderable controls and independent actions remains open, along
with live VoiceOver and system high contrast.

[Material 3 list accessibility](https://m3.material.io/components/lists/accessibility) ·
[Google Lists export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/6d4033c2-f92c-4c43-b585-b3bce8468d48.json) ·
[Surface #37](https://github.com/faberline/surface/issues/37)
