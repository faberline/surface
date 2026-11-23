# Surface v0.1.27 Lists verification

Check out `v0.1.27`. The base was `9fac6a0f7995f788d54757cf6a3a033d45585671`
(v0.1.26). Set `evidence_dir` to the absolute path of this folder. Run the
commands from the release checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-md3-lists-replay
scratch_dir=$(mktemp -d /tmp/surface-md3-lists-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native offscreen Metal

Native headless Metal reproduced 7 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.

The Gallery exposes Baseline Standard, Expressive Standard, and Expressive
Segmented. The replay covers passive rows, actions and controlled single
selection. It reaches a disabled row's neighbors by keyboard, checks the
3 dp focus stroke and 2 dp gap, wraps to a clipped row, and confirms focus
and selection separately. At 320 dp, dark RTL and larger text retain the
readable slots. The long selected name wraps inside the footer.
The 320 × 240 replay scrolls the scene to a selected row.

[Native report](native/native-list-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Offscreen GPU tests](native/gpu-offscreen-tests.log)

[Light Segmented](native/segmented-light.png) ·
[Focus outline](native/segmented-focused.png) ·
[Selection after wrap](native/segmented-wrap-selected.png) ·
[Dark Standard action](native/standard-dark-action.png) ·
[Baseline passive](native/baseline-light-passive.png) ·
[Dark RTL at 200%](native/segmented-compact-dark-rtl.png) ·
[Short dark RTL](native/segmented-short-dark-rtl.png)

```sh
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
python3 -B "$evidence_dir/native/replay.py" --workspace "$PWD" \
    --source-manifest "$evidence_dir/native/native-source.sha256" \
    --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native"
mkdir -p "$scratch_dir/gpu"
SURFACE_GPU_EVIDENCE="$scratch_dir/gpu/gpu-offscreen.png" \
    cap cargo test --locked -p surface-wgpu --test it gpu_offscreen -- --ignored --nocapture
```

## Headless Chrome WebGPU

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 9,815 checks. Material passed 165 steps and 9,567 checks.

M161–M166 inspect list and option roles, checked state and descriptions.
They send real keyboard, mouse and touch input. They verify the row
geometry, disabled and gap behavior, focus reveal, adaptive RTL text,
and the selected shape transition with and without Reduce Motion.

[Browser report](web/browser-material-lists.json) ·
[Browser suite results](web/headless-results.txt) ·
[Dark RTL capture](web/material-lists-compact-dark-rtl-large-text.png)

Install the wasm32 Rust target, matching wasm-bindgen CLI, wasm-opt, and
headless Chrome. GitHub Actions uses Rust 1.96.1, wasm-bindgen 0.2.129 and
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

## Reference and remaining coverage

Google's `md.comp.list` set is revision `2fdd`. The page export SHA-256 is
`0302dfe77765c4bd0aa25246b13bb8fddd561d3a10162256adc13a1eda226812`.
The token export SHA-256 is
`f9e0966358c53123286e2c9f3539d2962ccef39349d85857552c02d629f0ed27`.
The official keyboard table describes wrapping at the ends of the list.

Multi-select, multiple actions per row, expandable and reorder controls,
media layouts, ripple and timed hover or press shape changes remain open.
The browser and AccessKit projection tests exercise accessible names and
roles. Live VoiceOver speech and system high contrast remain open.

[Material 3 list specs](https://m3.material.io/components/lists/specs) ·
[Material 3 list accessibility](https://m3.material.io/components/lists/accessibility) ·
[Google token export](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.6c818a16475113bd.json) ·
[Surface #37](https://github.com/faberline/surface/issues/37)
