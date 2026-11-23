# Surface v0.1.22 Time Input verification

Check out tag `v0.1.22`. The starting commit for this batch was
`b9fcf9443a95f42db7b67b0132b2374fa5d3b27e` (v0.1.21).
Set `evidence_dir` to the absolute path to this folder. Run from the checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-time-input-replay
scratch_dir=$(mktemp -d /tmp/surface-md3-time-input-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native headless Metal

Native headless Metal reproduced 5 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.

The replay checks 12-hour entry and midnight-to-noon selection. It verifies
24-hour range errors, localized digits and explicit Save. It also checks both
helper texts in a dark 320 × 240 RTL dialog at 200 percent text.
The controls use DevTools semantic actions. The window check uses CoreGraphics.

[Native report](native/native-time-input-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Five offscreen Metal tests](native/gpu-offscreen-tests.log)

[Inline light midnight](native/inline-light-midnight.png) · [Modal dark rtl 200](native/modal-dark-rtl-200.png) · [Modal light independent error](native/modal-light-independent-error.png) · [Modal light japanese](native/modal-light-japanese.png) · [Modal light valid](native/modal-light-valid.png)

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

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 7,932 checks. Material passed 137 steps and 7,684 checks.

M134–M138 check the Chinese and Japanese labels and digits, keyboard selection,
field errors in the Chrome accessibility tree, composition Enter, explicit Save,
pointer dismissal, and focus restoration. The short dark dialog also checks
scrolling and sampled pixel contrast of the text, focus ring and error.

[Time Input report](web/browser-material-time-input.json) ·
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

The browser IME check uses synthetic composition. Live VoiceOver speech,
operating system input methods and system high contrast did not run.
The bundled fonts cover the Gallery's Chinese and Japanese fixtures.
Rendering Arabic text requires a font with Arabic glyphs. Unit tests
check Arabic-Indic digit parsing and normalization.

[Google time picker design](https://m3.material.io/components/time-pickers/specs) ·
[Published token export](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.39f4b152dd9bd292.json) ·
[Surface #21](https://github.com/faberline/surface/issues/21)
