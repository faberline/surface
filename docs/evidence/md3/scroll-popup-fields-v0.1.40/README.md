# Surface v0.1.40 anchored Scroll Field popup verification

Run from a checkout of `v0.1.40`. The base was
`1ad7aece8b23f0367e8398b43787534efa4323ae` (v0.1.39).
Set `evidence_dir` to this folder and `CARGO_TARGET_DIR` to a build
path in `/tmp`.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-md3-scroll-popup-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native headless Gallery

Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.

The light popup opens below its button and accepts independent values.
The dark RTL popup opens above its button. Outside actions remain
available while it is open. Closing discards the remaining drafts.
The compact 200% text view scrolls both editors past a fixed Close.
The replay checks focus, paint and the visible window count.

[Native report](native/native-scroll-popup-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Focused Rust tests](native/material-focused.log) ·
[Runtime adjustment tests](native/surface-app-adjustable.log) ·
[macOS AccessKit tests](native/macos-accessibility.log)

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

Chrome/158.0.8127.37 used Apple Metal 3. Five headless cases passed 14,633 checks. Material passed 233 steps and 14,385 checks.

M229–M234 exercise the named modeless dialog, pointer and keyboard
opening, retained outside focus, independent commits, Escape and active
synthetic composition, RTL flip, and compact scrolling with a fixed
Close button. The short inline M228 check also measures the full focus
ring and its lower painted stroke after Tab reveal.
The initial compact capture showed the outside Gallery button clipped.
The shorter visible label fits in the same row as the opener. Both
controls remain visible at 320×400 dp and 200% text size.

[Initial compact capture](web/pre-fix-compact-outside.png) · [20 dp to 0 dp measured clip](web/compact-clip-geometry.json)

To repeat the comparison, build the Gallery again with the original
outside label `外部操作 {n} 次`. Pass the old and new Web bundle
directories followed by a scratch output directory to
`node "$evidence_dir/web/compare-compact-clip.mjs"`.

[Browser report](web/browser-material-scroll-popup-fields.json) ·
[Browser commands](web/replay.md) ·
[Compact popup](web/material-scroll-popup-fields-compact-dark-rtl.png) ·
[Short inline focus ring](web/material-scroll-pair-fields-short-unit-focused.png)

## Design references

The dated Google page export has SHA-256
`3d7d5d816080d044cfc1a67cc7c33b7533cd3e91682ea6f99ab43e6c36c5a390`.
Google prefers inline placement and shows a modeless popup when space
is limited. Surface uses a named panel, leaves outside controls
available and gives each field its own draft. Modal placement,
system high contrast, real input-method entry, and live speech are open.

[Material 3 specs](https://m3.material.io/components/scroll-fields/specs) ·
[Published page export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/1624d6a1-96ea-437d-b846-dc6f00ebb49c.json) ·
[WAI-ARIA dialog role](https://www.w3.org/TR/wai-aria-1.2/#dialog) ·
[Surface #210](https://github.com/faberline/surface/issues/210)
