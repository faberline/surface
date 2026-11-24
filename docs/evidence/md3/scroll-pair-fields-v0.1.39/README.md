# Surface v0.1.39 quantity and unit Scroll Fields verification

Run from a checkout of `v0.1.39`. The base was
`0f31cab083d9a2f425501422197eff724b366c40` (v0.1.38).
Set `evidence_dir` to this folder and `CARGO_TARGET_DIR` to a build
path in `/tmp`.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-md3-scroll-pair-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native headless Gallery

Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.

The replay checks the light row and its independently adjustable values.
The dark RTL row mirrors the two fields and keeps quantity first in
keyboard order. The 320 dp dark RTL view at 200% text stacks the editors.
Both accept input and keep the other accepted value. The replay also
checks paint, focus restore, and the visible window count.

[Native report](native/native-scroll-pair-report.json) ·
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

Chrome/158.0.8127.37 used Apple Metal 3. Five headless cases passed 14,279 checks. Material passed 227 steps and 14,031 checks.

M223–M228 check the named group and its two values in Chrome's
accessibility tree. They exercise separate commits, disabled controls,
RTL row order, the 320 dp stack, and active synthetic composition
through resize. In a short viewport Tab reveals the next editor and
Back restores focus to the Gallery launcher.
At 320×360 dp, the entire unit editor becomes visible. Its outer focus
ring can meet the scroller edge and lose the lower band. The top and
sides of the ring remain visible. This edge remains in #210.

[Browser report](web/browser-material-scroll-pair-fields.json) ·
[Browser commands](web/replay.md) ·
[Compact capture](web/material-scroll-pair-fields-compact-dark-rtl.png)

## Design references

The dated Google page export has SHA-256
`3d7d5d816080d044cfc1a67cc7c33b7533cd3e91682ea6f99ab43e6c36c5a390`.
Google describes fields for paired measurements and calls for clear labels
and assistive selection or entry in both fields. Surface uses independent
state and a row that can stack. The app owns the meaning of the selected
number and unit. Popup and modal placement, system high contrast, real
input-method entry, and live speech are still open.

[Material 3 specs](https://m3.material.io/components/scroll-fields/specs) ·
[Published page export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/1624d6a1-96ea-437d-b846-dc6f00ebb49c.json) ·
[WAI spinbutton pattern](https://www.w3.org/WAI/ARIA/apg/patterns/spinbutton/) ·
[Surface #210](https://github.com/faberline/surface/issues/210)
