# Surface v0.1.20 Date Range Picker verification

Check out `v0.1.20`. The unchanged base for this batch was
`c64c49f0d5c0c13a92c6ee3b9fc8272e8e103560` (v0.1.19).
Set `evidence_dir` to this folder. Run the commands from the checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-date-range-picker-replay
scratch_dir=$(mktemp -d /tmp/surface-date-range-picker-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

Native headless Metal reproduced 6 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.

At 320 × 240 dp in RTL with 200% text, two consecutive month jumps
keep the Next button focused and fully visible. The new month aligns
to the clipped body each time. Tab reveals each date input in full.

[Native report](native/native-date-range-picker-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Cross-month range](native/calendar-light-crossing.png) ·
[320 dp dark RTL at 200%](native/calendar-dark-rtl-200.png) ·
[Reversed range error](native/input-light-reversed.png) ·
[Valid input](native/input-light-valid.png) ·
[Short calendar](native/calendar-dark-rtl-200-short.png) ·
[Short input](native/input-dark-rtl-200-short.png) ·
[Five offscreen Metal tests](native/gpu-offscreen-tests.log)

```sh
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
python3 -B "$evidence_dir/native/replay.py" --workspace "$PWD" \
    --source-manifest "$evidence_dir/native/native-source.sha256" \
    --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" \
    --out "$scratch_dir/native"
```

## Headless Chrome WebGPU

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 7,238 checks. Material passed 126 steps and 6,990 checks.

M122–M127 check range paint and accessibility, month and year keys,
draft and Save behavior, independent errors, composition, bounded
windowing and trusted gestures. The short case also checks a live
announcement while the painted summary is outside the clip.

[Range report](web/browser-material-date-range-picker.json) ·
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

The browser IME check uses synthetic composition events. Live VoiceOver
and system high contrast remain open. See Surface #20 for the remaining
date picker behavior.

[Google date picker design](https://m3.material.io/components/date-pickers/specs) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.01c5310367774eb8.json) ·
[Surface #20](https://github.com/faberline/surface/issues/20)
