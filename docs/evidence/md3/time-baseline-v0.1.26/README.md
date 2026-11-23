# Surface v0.1.26 Baseline time picker verification

Check out `v0.1.26`. The starting commit was
`6191afb45b8c484d90359697b87815361fa900ef` (v0.1.25).
Set `evidence_dir` to the absolute path to this folder. Run the
commands from the release checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-time-baseline-replay
scratch_dir=$(mktemp -d /tmp/surface-md3-time-baseline-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native offscreen Metal

Native headless Metal reproduced 7 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.

The replay verifies Baseline Input focus paint, 96 × 72 dp editors,
12-hour AM/PM, Japanese full-width 24-hour values, and Save focus
restoration. Baseline Dial uses a 256 dp face and separate period
targets. Portrait 24-hour mode uses 114 dp wide fields. Landscape RTL
mirrors the full face. At 320 × 240, dark RTL Japanese text at 200%
uses Baseline Input and keeps both action targets in view.

[Native report](native/native-time-baseline-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Five real GPU tests](native/gpu-offscreen-tests.log)

[Dial compact dark rtl japanese input](native/dial-compact-dark-rtl-japanese-input.png) · [Dial dark landscape rtl 12h](native/dial-dark-landscape-rtl-12h.png) · [Dial light portrait 12h](native/dial-light-portrait-12h.png) · [Dial light portrait 24h](native/dial-light-portrait-24h.png) · [Input dark japanese 24h](native/input-dark-japanese-24h.png) · [Input light hour focused](native/input-light-hour-focused.png) · [Input light minute focused](native/input-light-minute-focused.png)

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

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 9,427 checks. Material passed 159 steps and 9,179 checks.

M155–M160 inspect light and dark palette pixels, accessible focus,
both modal orientations and clock formats, real pointer and touch
choices, one shared draft, composition protection, the measured
compact action row, and Escape, Cancel and scrim dismissal.

[Browser report](web/browser-material-time-baseline.json) ·
[Browser suite results](web/headless-results.txt) ·
[Compact dark RTL Input](web/material-time-baseline-compact-dark-rtl-input.png)

Install the wasm32 Rust target, the matching wasm-bindgen CLI and
wasm-opt. GitHub Actions uses Rust 1.96.1, wasm-bindgen 0.2.129 and
Binaryen 133.

```sh
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build showcase --release --no-compress
cap cargo xtask web build material-gallery --release --no-compress
cap cargo xtask web build material-gallery --release --features devtools \
    --out "$CARGO_TARGET_DIR/web/devtools/material-gallery" --no-compress
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
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

The native replay uses semantic DevTools input. The browser composes
through the actual textarea with Chrome's test input API. Live
VoiceOver, a system IME and system high contrast require separate
checks.

The reference uses Google's `fe98` and `edb4` Baseline token sets
from revision `7989`. The official layout gives a smaller painted
AM/PM group. The implementation retains that paint at normal scale
and reserves separate 48 dp targets around it.

[Material time picker specs](https://m3.material.io/components/time-pickers/specs) ·
[Material touch target guidance](https://m3.material.io/components/time-pickers/accessibility) ·
[Google token export](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.39f4b152dd9bd292.json) ·
[Surface #21](https://github.com/faberline/surface/issues/21)
