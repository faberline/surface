# Surface v0.1.17 FAB Menu verification

The baseline is Surface v0.1.16 at
`c2c2f3de02d791d7557d9db15830f35d4f422236`.
Check out `v0.1.17` to replay. Set `evidence_dir` to this folder
and run the commands from the source checkout.

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

The desktop Gallery checks all 36 combinations of three sizes, three
palettes, two themes, and two action counts. It measures the fixed
top trailing anchor, the 56 dp close button and action geometry.
It checks the Web menu's 4 dp gap on both sides of its anchor and
its keyboard focus behavior. A 320 × 360 dp profile checks dark RTL
at 200% Chinese text. It verifies that the nearest action starts visible. It checks wrapping, contained scrolling,
the fixed close button and keyboard activation of an enabled item.

Both profiles reported zero visible windows before and after.
Two runs reproduced all ten PNGs and the JSON report byte for byte.
Page.screenshot, the CLI and MCP agreed on each PNG and its metadata.

[Native report](native/native-fab-menu-report.json) ·
[Replay comparison](native/native-replay-comparison.json) ·
[Light floating menu](native/native-light-primary.png) ·
[Web menu above its anchor](native/web-light-bottom.png) ·
[Compact RTL after scrolling](native/native-dark-rtl-200-scrolled.png) ·
[Five Metal renderer tests](native/gpu-offscreen-tests.log)

Build and replay with native Metal available.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-fab-menu-replay
scratch_dir=$(mktemp -d /tmp/surface-fab-menu-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
for attempt in 1 2; do
    python3 -B "$evidence_dir/native/verify-fab-menu-native.py" \
        --source "$source_digest" --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
        --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-$attempt"
done
diff -qr "$scratch_dir/native-1" "$scratch_dir/native-2"
SURFACE_GPU_EVIDENCE="$scratch_dir/gpu-offscreen.png" \
    cap cargo test --locked -p surface-wgpu --test it gpu_offscreen -- --ignored --nocapture
```

## Headless Chrome WebGPU

M106–M111 check one stable trigger, Web placement and dismissal,
floating shape and color, accessibility and keyboard order, and
outside touch. They verify painted focus and wrapped text at 320 dp
in dark RTL at 200% text. They also verify staggered entry and exit,
reopening during exit, and Reduce Motion. The implementation uses
the pinned Compose spring settings. Surface rounds the animated item
count so each action has its own reveal and removal threshold.

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed
6,532 checks. Material passed 110 steps and 6,284 checks.

[Browser report](web/browser-material-fab-menu.json) ·
[Five suites](web/headless-results.txt) ·
[Web compact focus](web/material-fab-menu-web-dark-rtl-200-focused.png) ·
[Floating compact focus](web/material-fab-menu-floating-dark-rtl-200-focused.png)

### Replay all five headless suites

Use Apple Metal and Chrome for a replay comparable to the local run.
Install the `wasm32-unknown-unknown` Rust target and put the matching
`wasm-bindgen` CLI and `wasm-opt` on `PATH`. The hosted workflow uses
Rust 1.96.1, wasm-bindgen 0.2.129 and Binaryen 133.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-fab-menu-replay
scratch_dir=$(mktemp -d /tmp/surface-fab-menu-replay.XXXXXXXX)

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

Chrome and the native Gallery start in headless mode. The hosted
workflow also runs three suites in a virtual Xvfb display.
System high contrast, physical touch and live screen-reader speech
still need platform checks.

[Official FAB Menu specs](https://m3.material.io/components/fab-menu/specs) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.1a076cec8bc202c2.json) ·
[FAB Menu accessibility](https://m3.material.io/components/fab-menu/accessibility) ·
[Pinned Compose FAB Menu](https://github.com/androidx/androidx/blob/ffbddbd1295b99a57b395a5dcfb86af31f451492/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/FloatingActionButtonMenu.kt)
