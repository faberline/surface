# Surface v0.1.23 Time Dial verification

Check out `v0.1.23`. The starting commit for this batch was
`31974c893f2f14d490ad536d1c45e2cb7795b90e` (v0.1.22).
Set `evidence_dir` to the absolute path to this folder. Run from the checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-time-dial-replay
scratch_dir=$(mktemp -d /tmp/surface-md3-time-dial-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native headless Metal

Native headless Metal reproduced 6 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.

The native replay checks the full 256 dp face in 390 × 720 and 844 × 390
viewports. It covers both clock formats, the checked hour and minute,
Japanese full-width digits, and the shared text draft.
It checks the named compact fallback at 320 × 300 with RTL and 200%
text, plus both dialog actions at that size.
In a larger window at 200% text, it checks separation of the two
Japanese hour rings and confirms all 24 options keep their names.
The replay uses DevTools semantic actions and CoreGraphics window counts.
Native Runtime tests also exercise point motion, cancellation and focus.

[Native report](native/native-time-dial-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Five offscreen Metal tests](native/gpu-offscreen-tests.log)

[Compact dark rtl](native/compact-dark-rtl.png) · [Horizontal dark japanese input](native/horizontal-dark-japanese-input.png) · [Horizontal dark rtl](native/horizontal-dark-rtl.png) · [Magnified dark rtl japanese](native/magnified-dark-rtl-japanese.png) · [Portrait light 12h](native/portrait-light-12h.png) · [Portrait light minute](native/portrait-light-minute.png)

```sh
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
python3 -B "$evidence_dir/native/replay.py" --workspace "$PWD" \
    --source-manifest "$evidence_dir/native/native-source.sha256" \
    --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" \
    --out "$scratch_dir/native"
mkdir -p "$scratch_dir/gpu"
SURFACE_GPU_EVIDENCE="$scratch_dir/gpu/gpu-offscreen.png" \
    cap cargo test --locked -p surface-wgpu --test it gpu_offscreen -- --ignored --nocapture
```

## Headless Chrome WebGPU

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 8,220 checks. Material passed 143 steps and 7,972 checks.

M139–M144 check the portrait face and preview, both hour rings and
exact minutes in landscape, projected keyboard and accessibility state,
trusted browser capture loss, shared invalid drafts and the compact
Input fallback with explicit return. The capture-loss receipt keeps the
trusted and synthetic event traces and the draft before and after loss.
An exact minute can overlap a printed mark. The test checks that the
overlapping ink clears while the mark keeps its accessibility option.

[Time Dial report](web/browser-material-time-dial.json) ·
[Browser suite results](web/headless-results.txt)

Install the wasm32 Rust target and put the matching wasm-bindgen CLI
and wasm-opt on PATH. GitHub Actions uses Rust 1.96.1,
wasm-bindgen 0.2.129 and Binaryen 133.

```sh
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build showcase --release --no-compress
cap cargo xtask web build material-gallery --release --no-compress
cap cargo xtask web build material-gallery --release --features devtools \
    --out "$CARGO_TARGET_DIR/web/devtools/material-gallery" --no-compress
cap cargo build --locked -p surface-material-gallery --features devtools --bin surface-material-gallery
cap cargo build --locked -p surface-devtools --bin surface-devtools
cap cargo build --locked -p jet-bridge --bin jet-bridge
unset SURFACE_CHROME_ARGS
cap "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --chrome-arg --headless=new \
    --web-root "$CARGO_TARGET_DIR/web/release" \
    --bridge-web "$CARGO_TARGET_DIR/web/devtools/material-gallery" \
    --jet-bridge "$CARGO_TARGET_DIR/debug/jet-bridge" \
    --devtools-bin "$CARGO_TARGET_DIR/debug/surface-devtools" \
    --native-bin "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --artifacts "$scratch_dir/chrome" \
    smoke ime material bridge-web devtools-headless
```

Live VoiceOver speech, hardware input methods and system high contrast
remain open. Browser composition uses synthetic events. The dial gives
immediate feedback. Animated selection movement remains separate.

[Google time picker design](https://m3.material.io/components/time-pickers/specs) ·
[Published token export](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.39f4b152dd9bd292.json) ·
[Surface #21](https://github.com/faberline/surface/issues/21)
