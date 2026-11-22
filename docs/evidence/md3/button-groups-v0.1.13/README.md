# Surface v0.1.13 button group verification

This batch starts from Surface v0.1.12
(`7fec533c20c53fe3ee2ac50f89b231896936c08a`). Check out
`v0.1.13` to reproduce the release. Set `evidence_dir` to this
folder. Run the manifest checks from the source tree.

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

The Gallery verifies 40 combinations of five sizes, two group types,
two outer shapes and both themes. The compact dark profile uses RTL and
200% Chinese text at 320 dp. Its semantic key sequence starts on the
first enabled item even when the second item is selected. Arrow keys
move focus and skip the disabled item. Enter and Space control selection.

The replay compares actual PNG paint and geometry with the exported
layout. `Page.screenshot`, the CLI and MCP return identical bytes and
metadata. Two separate replays produced nine identical PNGs and
identical JSON. CoreGraphics counted zero onscreen Gallery windows.

[Native report](native/native-button-group-report.json) ·
[Replay comparison](native/native-replay-comparison.json) ·
[Standard XS](native/light-standard-round-xs.png) ·
[Connected XL](native/dark-connected-square-xl.png) ·
[Dark RTL 200%](native/dark-rtl-200.png)

Use a Cargo target under `/tmp/cargo-target` and an environment that
can use Metal.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-button-group-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
PYTHONDONTWRITEBYTECODE=1 python3 "$evidence_dir/native/verify-button-group-native.py" \
    --source "$source_digest" --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-results"
```

## Headless Chrome WebGPU

Chrome 157 on Apple Metal 3 passed 87 Material steps and 4,436 checks.
The six button group steps account for 350 checks.

M83–M88 exercise actual color, shape, width and keyboard behavior.
A trusted Standard press takes width from the adjacent buttons and
keeps the bounds of the row. Cancellation restores its width.
Reduce Motion keeps the widths stable. Connected presses change the
inner contour. The browser checks selection, RTL and command overflow.
When widening moves a focused command inline, focus follows that same
command without activating it.

[Browser report](web/browser-material-button-group.json) ·
[Standard held width](web/material-button-group-standard-mouse-hold.png) ·
[Connected pressed contour](web/material-button-group-connected-pressed.png) ·
[Compact overflow](web/material-button-group-compact-command-overflow.png)

```sh
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build material-gallery --release --no-compress
cap "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --web-root "$CARGO_TARGET_DIR/web/release" \
    --artifacts "$scratch_dir/web-results" material
```

Live VoiceOver speech and physical device touch still need platform
testing. System high-contrast integration remains a shared foundation
gap.

[Material 3 button group specs](https://m3.material.io/components/button-groups/specs) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.082d7fd6e058b011.json)
