# Surface v0.1.21 Date Picker Interactions verification

Check out `v0.1.21`. The unchanged base for this batch was
`7567409f9c21f09b5eb0004ca631379f59459375` (v0.1.20).
Set `evidence_dir` to this folder. Run the commands from the checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-date-picker-interactions-replay
scratch_dir=$(mktemp -d /tmp/surface-date-picker-interactions-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

Native headless Metal reproduced 4 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.

The Gallery replay checks the modal calendar in light and dark RTL at
200% text, and the docked month and year selectors. The native Gallery
test drives touch through Surface Runtime. It checks the held preview,
the passive accessibility tree, cancel, one page turn and focus.
The focused Rust key tests exercise both shortcut focus destinations.
The headless DevTools replay uses its named semantic actions.

[Native report](native/native-date-interaction-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Five offscreen Metal tests](native/gpu-offscreen-tests.log)

[Docked month list](native/docked-month-list.png) · [Docked year list](native/docked-year-list.png) · [Single dark rtl 200](native/single-dark-rtl-200.png) · [Single light march](native/single-light-march.png)

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

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 7,575 checks. Material passed 132 steps and 7,327 checks.

M128–M133 check real touch swipes, held and canceled motion, clipping,
RTL direction, narrow-grid panning, the exact seam between the two preview pages, Reduce Motion, shortcut focus and
key ownership in the list and editor.

[Interaction report](web/browser-material-date-picker-gestures.json) ·
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
