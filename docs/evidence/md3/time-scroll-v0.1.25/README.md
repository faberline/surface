# Surface v0.1.25 Scroll time picker verification

Check out `v0.1.25`. This release started at
`19b48eab43ce1b472b8438fab02063877f8c84e4` (v0.1.24).
Set `evidence_dir` to the absolute path to this evidence folder.
Run the commands from the release checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-time-scroll-replay
scratch_dir=$(mktemp -d /tmp/surface-md3-time-scroll-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native offscreen Metal

Native headless Metal reproduced 6 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.

The replay uses four separate headless Gallery profiles. It checks
inline and modal layout, correct initial focus and range, 12-hour period
controls, independent keyboard adjustment, and dark RTL 24-hour mode
with Japanese full-width digits at 200% text. It opens direct Input,
checks the stable focus and edit description, enters a time, returns to
Scroll and saves. The 320 × 240 fallback preserves visible actions and
the disabled profile leaves Cancel available.

[Native report](native/native-time-scroll-report.json) ·
[Two replay comparison](native/native-repeat.json) ·
[Five real GPU tests](native/gpu-offscreen-tests.log) ·
[AccessKit value and description](native/accesskit-adjustable-tests.log)

[Compact dark rtl japanese input](native/compact-dark-rtl-japanese-input.png) · [Disabled light](native/disabled-light.png) · [Inline light 12h](native/inline-light-12h.png) · [Modal dark rtl direct input](native/modal-dark-rtl-direct-input.png) · [Modal dark rtl japanese 24h](native/modal-dark-rtl-japanese-24h.png) · [Modal light 12h](native/modal-light-12h.png)

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

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 8,882 checks. Material passed 153 steps and 8,634 checks.

M149–M154 exercise independent wheel input bursts, touch and capture
loss, adjacent taps, keyboard focus and accessible values, the missing
value description, dark RTL at 200% text, and a 320 × 240 Input fallback.
Synthetic browser composition tests Enter. Save flushes a pending wheel
selection once. Cancel, Escape and the scrim leave the saved time intact.

[Scroll behavior report](web/browser-material-time-scroll.json) ·
[Browser suite results](web/headless-results.txt) ·
[Dark RTL Japanese modal](web/material-time-scroll-modal-wide-dark-rtl.png) ·
[Compact Input](web/material-time-scroll-compact-dark-rtl-input.png)

Install the wasm32 Rust target and put the matching wasm-bindgen CLI
and wasm-opt on PATH. GitHub Actions uses Rust 1.96.1,
wasm-bindgen 0.2.129 and Binaryen 133.

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

The native transport replay uses semantic DevTools input. Rust Runtime
tests exercise captured pointer input and the macOS AccessKit tree.
Live VoiceOver, a system IME and system high contrast remain open.

[Material 3 time picker design](https://m3.material.io/components/time-pickers/specs) ·
[Material 3 time picker accessibility](https://m3.material.io/components/time-pickers/accessibility) ·
[Surface #21](https://github.com/faberline/surface/issues/21)
