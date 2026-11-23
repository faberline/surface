# Surface v0.1.19 Docked Date Picker verification

Check out `v0.1.19` to replay. The baseline was
`f72a23b8422341f979605ebc605f3a2a00269551` (v0.1.18).
Set `evidence_dir` to this folder and run the commands from the checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-docked-date-picker-replay
scratch_dir=$(mktemp -d /tmp/surface-docked-date-picker-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

The Gallery checks the 360 × 456 dp panel below and above the field,
live field state, month and year lists, selected, current and unavailable
days, typed leap dates, focus exit validation, and 320 dp RTL at 200%
Chinese text. Two runs match six PNGs and the report byte for byte.
The screenshot API, CLI and MCP agree on each image and its metadata.
CoreGraphics reports zero visible Gallery windows.

[Native report](native/native-docked-date-picker-report.json) ·
[Replay comparison](native/native-replay-comparison.json) ·
[Light calendar below](native/calendar-light-below.png) ·
[Dark calendar above](native/calendar-dark-above.png) ·
[Compact RTL calendar](native/calendar-dark-rtl-200.png) ·
[Offscreen Metal tests](native/gpu-offscreen-tests.log)

```sh
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
for attempt in 1 2; do
    python3 -B "$evidence_dir/native/verify-docked-date-picker-native.py" \
        --source "$source_digest" --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
        --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-$attempt"
done
diff -qr "$scratch_dir/native-1" "$scratch_dir/native-2"
SURFACE_GPU_EVIDENCE="$scratch_dir/gpu-offscreen.png" \
    cap cargo test --locked -p surface-wgpu --test it gpu_offscreen -- --ignored --nocapture
```

## Headless Chrome WebGPU

M116–M121 check the separate field and calendar button, date entry
and validation, held pointer focus, composition, grid accessibility,
month and year browsing, dismissal, and compact RTL.
The composition evidence uses Chrome's synthetic IME events.

The first Linux run used a field position from before text reflow.
This caused the popup to overlap the calendar button. The Gallery now
requests a frame after layout changes and places the popup at the updated
field position. A Gallery test checks the first frame after opening.
[Original hosted failure geometry](web/hosted-anchor-before.json) ·
[Anchor comparison](web/anchor-geometry-comparison.json) ·
[Original GitHub Actions run](https://github.com/faberline/surface/actions/runs/45091274456)

Chrome/157.0.8103.79 used Apple Metal 3. All five cases passed 6,902 checks. Material passed 120 steps and 6,654 checks.

[Docked Date Picker report](web/browser-material-docked-date-picker.json) ·
[Five local suites](web/headless-results.txt)

Install the `wasm32-unknown-unknown` Rust target. Put the matching
`wasm-bindgen` CLI and `wasm-opt` on `PATH`. The hosted workflow uses
Rust 1.96.1, wasm-bindgen 0.2.129 and Binaryen 133.

```sh
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build showcase --release --no-compress
cap cargo xtask web build material-gallery --release --no-compress
cap cargo xtask web build material-gallery --release --features devtools \
    --out "$CARGO_TARGET_DIR/web/devtools/material-gallery" --no-compress
cap cargo build --locked -p surface-material-gallery --features devtools \
    --bin surface-material-gallery
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
    --artifacts "$scratch_dir/web-results" \
    smoke ime material bridge-web devtools-headless
```

The hosted workflow uses a virtual display for three additional suites.
Live VoiceOver, system high contrast and physical input still need
verification. Date ranges, swipe paging and the Shift+M and Shift+Y
keyboard shortcuts remain tracked in #20.

[Google docked date picker](https://m3.material.io/components/date-pickers/specs) ·
[Published date picker tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.01c5310367774eb8.json) ·
[Material accessibility](https://m3.material.io/components/date-pickers/accessibility) ·
[Surface #20](https://github.com/faberline/surface/issues/20)
