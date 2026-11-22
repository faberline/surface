# Surface v0.1.18 Date Picker verification

Check out Surface `v0.1.18` to replay these tests. The baseline was
`e004097015d1545b96b209ec7bf4012044bbc8b9` (v0.1.17).
Set `evidence_dir` to this evidence folder and run the commands from
the source checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-date-picker-replay
scratch_dir=$(mktemp -d /tmp/surface-date-picker-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

The Gallery checks the regular calendar and input sizes, selected-day
paint, disabled days, the separate today marker, year and month
navigation, focus return, typed date normalization and errors. A 320 dp
dark RTL profile keeps 48 dp date targets, Save and Close at 200%
Chinese text. Home and End reveal each edge cell. CoreGraphics
reported zero visible windows.

Two runs matched six PNGs and the report byte for byte. The screenshot
API, CLI and MCP agreed on each image and its metadata.

[Native report](native/native-date-picker-report.json) ·
[Replay comparison](native/native-replay-comparison.json) ·
[Calendar](native/calendar-light.png) ·
[Compact calendar](native/calendar-dark-rtl-200.png) ·
[Compact input](native/input-dark-rtl-200.png) ·
[Invalid date](native/input-invalid-dark.png) ·
[Offscreen Metal tests](native/gpu-offscreen-tests.log)

```sh
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
for attempt in 1 2; do
    python3 -B "$evidence_dir/native/verify-date-picker-native.py" \
        --source "$source_digest" --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
        --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-$attempt"
done
diff -qr "$scratch_dir/native-1" "$scratch_dir/native-2"
SURFACE_GPU_EVIDENCE="$scratch_dir/gpu-offscreen.png" \
    cap cargo test --locked -p surface-wgpu --test it gpu_offscreen -- --ignored --nocapture
```

## Headless Chrome WebGPU

M112–M115 check the Chrome accessibility tree, grid roles, date
selection, unavailable days, today, year pages, keyboard navigation,
dismissal, and descriptions on the real active textarea. The test
driver uses Chrome composition events to check that Enter during
composition does not commit the date. A separate Enter submits the
date after the composition ends.

Chrome/157.0.8103.79 used Apple Metal 3. All five cases passed 6,627 checks. Material passed 114 steps and 6,379 checks.

[Browser date picker report](web/browser-material-date-picker.json) ·
[Five cases](web/headless-results.txt)

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
The AccessKit tree has focused unit evidence. Live AppKit and VoiceOver
behavior, OS high contrast and physical input still need verification.
Docked pickers and date ranges remain tracked in issue #20.

[Google Date Picker](https://m3.material.io/components/date-pickers/specs) ·
[Published token table](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.01c5310367774eb8.json) ·
[Date picker keyboard pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/examples/datepicker-dialog/) ·
[Surface #20](https://github.com/faberline/surface/issues/20)
