# Surface v0.1.29 multi-action Lists verification

Check out `v0.1.29`. The base was `aa0cc1290068a0022fe5d98ac9c7091f3cbf1da2`
(v0.1.28). Set `evidence_dir` to the absolute path to this folder. Run each
command from the release checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-md3-list-actions-replay
scratch_dir=$(mktemp -d /tmp/surface-md3-list-actions-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native offscreen Metal

Native headless Metal reproduced 7 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.

The replay checks independent primary and supplementary actions in all three
styles. It verifies native list and button roles, separate focus, a 3 dp
inside stroke, the toggle state, disabled actions and four-arrow traversal
with wrap. The dark RTL scene wraps long text and moves the action strip
below the primary button. A 320 × 240 scene reveals and presses a full target.

[Native report](native/native-list-actions-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Offscreen GPU tests](native/gpu-offscreen-tests.log)

[Light Segmented](native/actions-segmented-light.png) ·
[Focused pin](native/actions-pin-focus.png) ·
[Focus after wrap](native/actions-wrap-focus.png) ·
[Dark Standard](native/actions-standard-dark.png) ·
[Light Baseline](native/actions-baseline-light.png) ·
[Dark RTL with 200% text](native/actions-compact-dark-rtl.png) ·
[Short dark RTL](native/actions-short-dark-rtl.png)

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

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 10,636 checks. Material passed 176 steps and 10,388 checks.

M172–M177 check named sibling actions, exact action and icon sizes, focus
entry and reveal, and independent keyboard commands. They send actual mouse
and touch input. Releasing on a different action or in a gap runs neither
command. At 320 dp with RTL and 200% text, all actions stay reachable, and
the same stable action retains focus when the row reflows on resize.

[Browser report](web/browser-material-list-actions.json) ·
[Browser suite results](web/headless-results.txt) ·
[Dark RTL capture](web/material-list-actions-compact-dark-rtl-reflow.png)

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

Google's `md.comp.list` token set is revision `2fdd`. The page export SHA-256 is
`0302dfe77765c4bd0aa25246b13bb8fddd561d3a10162256adc13a1eda226812`.
The token export SHA-256 is
`f9e0966358c53123286e2c9f3539d2962ccef39349d85857552c02d629f0ed27`.

Expandable and reorder controls, media layouts, ripple, timed hover or
press shape changes, live VoiceOver speech and system high contrast
remain open.

[Material 3 list specs](https://m3.material.io/components/lists/specs) ·
[Material 3 list accessibility](https://m3.material.io/components/lists/accessibility) ·
[Google token export](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.6c818a16475113bd.json) ·
[Surface #37](https://github.com/faberline/surface/issues/37)
