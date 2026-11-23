# Surface v0.1.28 multi-select Lists verification

Check out `v0.1.28`. The base was `18575735454e4a175e3a2af43d34c24eda4a78f8`
(v0.1.27). Set `evidence_dir` to the absolute path of this folder. Run the
commands from the release checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-md3-lists-multi-replay
scratch_dir=$(mktemp -d /tmp/surface-md3-lists-multi-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native offscreen Metal

Native headless Metal reproduced 7 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.

The list starts with two checks. Space adds one other row. Enter removes
one check from the end of the list. Arrow movement skips disabled rows and
wraps without changing a check. The native projection test verifies separate
checkbox actions, names and state. The replay checks the 18 dp checkbox
inside a full row, the outline and fill, and the same scene in all three
styles. At 320 dp, dark RTL and 200% text keep the long row readable. A
320 × 240 scene can reveal the first checked row and activate it with Enter.

[Native report](native/native-list-multi-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Offscreen GPU tests](native/gpu-offscreen-tests.log)

[Initial checks](native/multi-segmented-light-start.png) ·
[Third row added](native/multi-segmented-light-three-checked.png) ·
[One row unchecked](native/multi-segmented-light-last-deselected.png) ·
[Dark Standard](native/multi-standard-dark.png) ·
[Light Baseline](native/multi-baseline-light.png) ·
[Dark RTL with 200% text](native/multi-compact-dark-rtl.png) ·
[Short dark RTL](native/multi-short-dark-rtl.png)

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

Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 10,181 checks. Material passed 170 steps and 9,933 checks.

M167–M171 verify the named multi-select listbox and the selected state of
its options in Chrome's accessibility tree. They use keyboard, real mouse
and touch input. They check held-key repeat suppression, independent
selections, disabled rows,
checkbox paint, light and dark styles, stable targets during selection,
and narrow RTL with 200% text.

[Browser report](web/browser-material-lists-multi.json) ·
[Browser suite results](web/headless-results.txt) ·
[Dark RTL capture](web/material-lists-multi-compact-dark-rtl-large-text.png)

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

Multiple actions per row, expandable and reorder controls, media layouts,
ripple, timed hover or press shape changes, live VoiceOver speech and
system high contrast remain open.

[Material 3 list specs](https://m3.material.io/components/lists/specs) ·
[Material 3 list accessibility](https://m3.material.io/components/lists/accessibility) ·
[AndroidX CheckboxTokens 14_1_0](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/CheckboxTokens.kt) ·
[WAI-ARIA listbox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/) ·
[Google token export](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.6c818a16475113bd.json) ·
[Surface #37](https://github.com/faberline/surface/issues/37)
