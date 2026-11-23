# Surface v0.1.36 Scroll Fields verification

Run from a checkout of `v0.1.36`. The base was
`212e52b128784200993b38c2a4cded0e82bf9331` (v0.1.35).
Set `evidence_dir` to this folder and `CARGO_TARGET_DIR` to a build
directory in /tmp.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-md3-scroll-fields-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native headless Gallery

Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.

The native Gallery checks the wheel and direct editor with Reduce Motion.
It uses real keyboard routing through DevTools, including Tab, arrow
adjustment and Enter on the focused editor. It also checks a full-width
draft, invalid state, error association, focus and 200% text. The
screen checks query each process's visible window count.

[Native report](native/native-scroll-field-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Focused Rust tests](native/material-focused.log) ·
[macOS AccessKit validation test](native/macos-accessibility.log)

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

Chrome/158.0.8127.37 used Apple Metal 3. Five headless cases passed 13,411 checks. Material passed 209 steps and 13,163 checks.

M205–M210 check all three size floors, contour and token pixels, real
48 dp neighbor regions outside the face, browser keyboard and pointer,
swipe preview, cancellation, settling, Reduce Motion and page scroll.
They verify the Chrome accessibility tree, invalid feedback and synthetic
composition Enter. A wide range triggers the direct editor at 320 dp in
dark RTL with 200% text. The editor preserves active preedit and the draft when the view becomes wider.

[Browser report](web/browser-material-scroll-fields.json) ·
[Browser commands](web/replay.md) ·
[Cropped target zones](web/material-scroll-fields-cropped-zones.png) ·
[Visible error](web/material-scroll-fields-editor-error.png) ·
[Spring sample](web/material-scroll-fields-gesture-spring.png) ·
[Compact fallback](web/material-scroll-fields-compact-dark-rtl.png) ·
[Preedit after resize](web/material-scroll-fields-resized-ime-preedit.png)

The initial headless proof caught a missing focus transfer on a center tap.
M206 now exercises the same physical tap and requires focus on the editor.
[Pre-fix receipt](web/pre-fix-center-tap.json) · [Pre-fix run log](web/pre-fix-center-tap.log)

A separate check reproduced the removal of a pristine fallback editor
while it held active preedit. M210 requires both active composition and
typed drafts to survive resize.
[Pre-fix resize receipt](web/pre-fix-preedit-resize.json)

## Design reference and remaining coverage

The dated Material page export has SHA-256
`3d7d5d816080d044cfc1a67cc7c33b7533cd3e91682ea6f99ab43e6c36c5a390`.
The token export has SHA-256
`5afcd56f8cb9fe47d822b4b14426db8b0ed27aa9d70e015d6c9389bf6d6500cd`.
The 120 dp Medium minimum comes from the token table and measurement
diagram. Guidelines prose lists 144 dp. Surface puts larger values
above the center so Up selects above and agrees with Increment.
Looping, text choices, independent units, popup and modal placement,
system high contrast, native OS composition and live speech remain open.

[Material 3 specs](https://m3.material.io/components/scroll-fields/specs) ·
[Published page export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/1624d6a1-96ea-437d-b846-dc6f00ebb49c.json) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.42f95ef205743188.json) ·
[Surface #210](https://github.com/faberline/surface/issues/210)
