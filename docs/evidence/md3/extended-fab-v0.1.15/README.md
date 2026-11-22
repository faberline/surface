# Surface v0.1.15 Extended FAB verification

This batch starts from Surface v0.1.14 at
`b555e7ed53751d020a543abeba4c3a347708cab8`. Check out
`v0.1.15` to reproduce the checks. Set `evidence_dir` to this
folder. Run the source checks from the code checkout.

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

The Gallery provides a scrollable notes view and a pinned main action.
The checks cover three sizes, six colors, two themes, and both forms
of the control. A second profile uses 320 dp, dark theme, RTL and
200% Chinese text. The replay checks keyboard focus, command activation,
the lowered shadow, disabled and hidden behavior, and text ink
clearance from the rounded face.

CoreGraphics counted zero visible windows. The two replays produced
18 identical PNGs. The API, CLI and MCP matched the bytes and metadata.

[Native report](native/native-extended-fab-report.json) ·
[Replay comparison](native/native-replay-comparison.json) ·
[Light focus](native/desktop-focus.png) ·
[Lowered elevation](native/light-lowered.png) ·
[Dark RTL expanded](native/dark-rtl-200-large.png) ·
[Dark RTL collapsed](native/dark-rtl-200-collapsed.png) ·
[Dark RTL text only](native/dark-rtl-200-text-only.png) ·
[Five Metal renderer tests](native/gpu-offscreen-tests.log)

Build and replay with native Metal available.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-extended-fab-replay
scratch_dir=$(mktemp -d /tmp/surface-extended-fab-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
for attempt in 1 2; do
    python3 -B "$evidence_dir/native/verify-extended-fab-native.py" \
        --source "$source_digest" --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
        --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-$attempt"
done
diff -qr "$scratch_dir/native-1" "$scratch_dir/native-2"
```

## Headless Chrome WebGPU

M95–M99 check actual paint, a stable accessible action, trusted pointer,
keyboard and touch input, scroll-driven width motion, controlled
visibility, motion reversal, Reduce Motion, and compact RTL text.

Chrome 157 used Apple Metal 3. All five local headless suites passed
5,544 checks. The Material case passed 98 steps and 5,296 checks.
The five new Extended FAB steps account for 424 checks.

[Browser report](web/browser-material-extended-fab.json) ·
[All five suites](web/headless-results.txt) ·
[Compact RTL](web/material-extended-fab-compact-dark-rtl-200.png) ·
[Wheel motion](web/material-extended-fab-wheel-morph-middle.png) ·
[Fade on exit](web/material-extended-fab-visibility-exit.png)

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
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-extended-fab-replay
scratch_dir=$(mktemp -d /tmp/surface-extended-fab-replay.XXXXXXXX)

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

Live screen-reader speech, system high contrast, and physical device
touch still need platform verification. Radial ripple and a
destination-surface transform remain shared gaps.

[Official Extended FAB specs](https://m3.material.io/components/extended-fab/specs) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.36500f77b86d20a5.json) ·
[Pinned Material Components motion](https://github.com/material-components/material-components-android/tree/66c334b7946dabf33adfe1a2b7cad6bcaa4ea3ad/lib/java/com/google/android/material/floatingactionbutton/res/animator)
