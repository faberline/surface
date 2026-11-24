# Surface v0.1.41 modal Scroll Fields verification

Run from a checkout of `v0.1.41`. The base was
`ccfc9c0e3c6f47c6e01ca045877d34e79256f285` (v0.1.40).
Set `evidence_dir` to this folder and `CARGO_TARGET_DIR` to a build
path in `/tmp`.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-md3-scroll-modal-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native headless Gallery

Native headless Metal reproduced 8 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.

The replay covers named focus containment and both Tab directions.
It reveals the two stacked wheels in their own focus frames. It checks
the scrim paint, rejects background access, and shows
that Save writes the two values together. It verifies cancellation,
validation, reopening, and the fixed footer at 320×400 dp and 200%
text. The input path uses semantic DevTools actions.

[Native report](native/native-scroll-modal-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Focused Material tests](native/material-final.log) ·
[Gallery tests](native/gallery-final.log) ·
[macOS AccessKit tests](native/macos-accessibility-final.log)

```sh
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
(
    cd "$CARGO_TARGET_DIR"
    shasum -a 256 debug/surface-material-gallery debug/surface-devtools
) > "$scratch_dir/binaries.sha256"
python3 -B "$evidence_dir/native/replay.py" --workspace "$PWD" \
    --source-manifest "$evidence_dir/native/native-source.sha256" \
    --binary-manifest "$scratch_dir/binaries.sha256" \
    --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native"
SURFACE_GPU_EVIDENCE="$scratch_dir/gpu/gpu-offscreen.png" sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --lib gpu_offscreen_raster_cache -- --ignored --nocapture
```

## Headless Chrome WebGPU

Chrome/158.0.8127.37 used Apple Metal 3. Five headless cases passed 15,298 checks. Material passed 239 steps and 15,050 checks.

M235–M240 cover the named modal scope, a single Save for both fields,
validation and its live summary, focus restoration, scrim pointer
routing, synthetic composition and RTL. At 320 dp and 200% text,
Cancel and Save remain visible during scroll. The shortest case
hides the dialog at 217 dp height and presents both drafts at 218 dp.
The real capture shows the whole focused editor and ring at that height.

[Browser report](web/browser-material-scroll-modal-fields.json) ·
[Browser commands](web/replay.md) ·
[Compact RTL capture](web/material-scroll-modal-fields-compact-dark-rtl.png) ·
[Shortest editor capture](web/material-scroll-modal-fields-shortest-editor.png) ·
[Initial clipped focus ring](web/initial/clipped-unit.png) · [Initial measurements](web/initial/geometry.json)

## Design references

The dated Google page export has SHA-256
`3d7d5d816080d044cfc1a67cc7c33b7533cd3e91682ea6f99ab43e6c36c5a390`.
It shows a modal for Scroll Fields where the interface is too crowded
for a modeless popup. The Gallery validates then saves the pair
together. System high contrast, live OS input and live speech remain open.

[Published Material 3 guidance](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/1624d6a1-96ea-437d-b846-dc6f00ebb49c.json) ·
[WAI-ARIA dialog role](https://www.w3.org/TR/wai-aria-1.2/#dialog) ·
[W3C description calculation draft](https://www.w3.org/TR/2026/WD-accname-1.2-20261104/#computation-steps) ·
[Surface #210](https://github.com/faberline/surface/issues/210)
