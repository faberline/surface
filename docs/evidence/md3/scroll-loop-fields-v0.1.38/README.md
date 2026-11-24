# Surface v0.1.38 looping Scroll Fields verification

Run from a checkout of `v0.1.38`. The base was
`1f85afea87291f75f1a88c5b45bbee8d5f0def9b` (v0.1.37).
Set `evidence_dir` to this folder and `CARGO_TARGET_DIR` to a build
path in `/tmp`.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-md3-scroll-loop-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native headless Gallery

Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.

The light numeric wheel crosses both ends by keyboard. The dark RTL
choice wheel uses two items and retains the selected ID when the catalog
reorders. The 320 dp dark RTL view at 200% text retains the editor and
accepts a localized alias. The replay checks paint, targets and focus.

[Native report](native/native-scroll-loop-report.json) ·
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

Chrome/158.0.8127.37 used Apple Metal 3. Five headless cases passed 14,005 checks. Material passed 221 steps and 13,757 checks.

M217–M222 exercise both seams, a held multi-lap drag, release and cancel,
cropped tap zones, page scrolling, the two directions of a two-item wheel,
catalog replacement, true accessibility bounds, and the compact editor.

[Browser report](web/browser-material-scroll-loop-fields.json) ·
[Browser commands](web/replay.md) ·
[Compact capture](web/material-scroll-loop-fields-compact-dark-rtl.png)

The first 2× run lost the upper cropped label. The fix expands the text
renderer’s run search and applies the exact paint clip at draw time.
The passing M220 checks the visible pixels on both sides.

[Before the fix](web/pre-fix-top-peek.png) · [Before-fix receipt](web/pre-fix-top-peek.json) · [After the fix](web/material-scroll-loop-fields-choice-two-peeks.png)

## Design references

The dated Google page export has SHA-256
`3d7d5d816080d044cfc1a67cc7c33b7533cd3e91682ea6f99ab43e6c36c5a390`.
Google allows a looping list. Surface wraps relative movement and keeps
its bounds for direct entry, absolute selection, Home and End. Adjacent
quantity and unit controls, popup and modal placement, system high
contrast, real input-method entry, and live speech remain open.

[Material 3 specs](https://m3.material.io/components/scroll-fields/specs) ·
[Published page export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/1624d6a1-96ea-437d-b846-dc6f00ebb49c.json) ·
[WAI spinbutton pattern](https://www.w3.org/WAI/ARIA/apg/patterns/spinbutton/) ·
[Surface #210](https://github.com/faberline/surface/issues/210)
