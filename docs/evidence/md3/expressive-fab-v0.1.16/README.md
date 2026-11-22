# Surface v0.1.16 Expressive FAB verification

The baseline is Surface v0.1.15 at
`b3a9d824da58e3e65acd6a27674fa934e6bf0707`. Check out
`v0.1.16` to reproduce this batch. Set `evidence_dir` to this
directory and run these commands from the source checkout.

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

The Gallery scene checks 36 size, theme and palette profiles.
It compares the lowered shadow, semantic action and keyboard input,
disabled behavior, focus after hiding, and the tooltip above its
focus ring. A second profile uses 320 dp, dark theme, RTL and 200%
Chinese text. Both profiles count their visible windows.

Both profiles reported zero visible windows before and after. The
two runs matched all 17 PNGs and the JSON report. API, CLI and MCP
matched each PNG's bytes and metadata.

[Native report](native/native-expressive-fab-report.json) ·
[Replay comparison](native/native-replay-comparison.json) ·
[Keyboard tooltip](native/light-keyboard-tooltip.png) ·
[Lowered elevation](native/light-lowered.png) ·
[Dark RTL 200%](native/dark-rtl-200-tooltip.png) ·
[Five Metal renderer tests](native/gpu-offscreen-tests.log)

Build and replay with native Metal available.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-expressive-fab-replay
scratch_dir=$(mktemp -d /tmp/surface-expressive-fab-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
for attempt in 1 2; do
    python3 -B "$evidence_dir/native/verify-expressive-fab-native.py" \
        --source "$source_digest" --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
        --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-$attempt"
done
diff -qr "$scratch_dir/native-1" "$scratch_dir/native-2"
```

## Headless Chrome WebGPU

M100–M105 check the Expressive face, actual clipped ripple pixels,
reduced motion, touch and keyboard activation, Chrome accessibility,
tooltip behavior, compact RTL placement during scroll, and
interrupted visibility motion. The trusted pointer and touch events in
this replay use Chrome's test driver.

Chrome 157 used Apple Metal 3. All five local suites passed
5,909 checks. Material passed 104 steps and 5,661 checks. The
six new FAB steps passed 365 checks.

[Browser report](web/browser-material-fab.json) ·
[Five suites](web/headless-results.txt) ·
[Early ripple](web/material-fab-ripple-early.png) ·
[Rounded clip](web/material-fab-ripple-clipped.png) ·
[Compact keyboard tooltip](web/material-fab-compact-dark-rtl-tooltip.png) ·
[Visibility exit](web/material-fab-visibility-leaving.png)

### Replay all five headless suites

Run this from the Surface source checkout named by the tested source commit.
Keep the evidence folder separate from that checkout. Use Apple Metal and
Chrome for a replay comparable to the local report. Install the
`wasm32-unknown-unknown` Rust target and put the matching `wasm-bindgen`
CLI and `wasm-opt` on `PATH`. The hosted workflow uses Rust 1.96.1,
wasm-bindgen 0.2.129 and Binaryen 133.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-expressive-fab-replay
scratch_dir=$(mktemp -d /tmp/surface-expressive-fab-replay.XXXXXXXX)

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

The browser cases and the Jet Bridge web case start Chrome headlessly.
The `devtools-headless` case starts the native Gallery with
`--headless`. The hosted workflow also runs the three windowed suites
under Xvfb.

System high contrast, physical touch and live screen-reader speech
still need platform verification. This component has a rounded-face
ripple. Adoption across the catalog remains open.

[Official FAB specs](https://m3.material.io/components/floating-action-button/specs) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.41587918e51cca98.json) ·
[Material FAB interaction and composition](https://m3.material.io/components/floating-action-button/guidelines) ·
[Pinned Compose FAB](https://github.com/androidx/androidx/blob/ffbddbd1295b99a57b395a5dcfb86af31f451492/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/FloatingActionButton.kt)
