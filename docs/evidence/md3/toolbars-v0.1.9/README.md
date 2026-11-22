# Surface v0.1.9 toolbar verification

This batch starts from Surface v0.1.8
(`9557229b4409a7ce0aa92e88d32e601751945c41`). From a matching source
checkout set `evidence_dir` to this folder and check both manifests.

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

The 960 × 800 replay checks three toolbar layouts in both palettes.
It verifies 48 dp action targets, directional focus, disabled and
toggle states, overflow isolation and focus return, and an adjacent FAB.
The 320 × 580 dark RTL profile checks both floating orientations,
compact paired placement and 200% Chinese text. It also compares the
raw screenshot API, CLI and MCP bytes. Both profiles use Reduce Motion.

[Native report](native/native-toolbar-report.json) ·
[Light docked](native/light-docked-standard.png) ·
[Compact paired FAB](native/dark-rtl-200-paired.png) ·
[Dark vertical](native/dark-rtl-200-vertical.png)

Use a Cargo target under `/tmp/cargo-target` and a command environment
with Metal access. The replay opens no window.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-toolbar-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
PYTHONDONTWRITEBYTECODE=1 python3 "$evidence_dir/native/verify-toolbar-native.py" \
    --source "$source_digest" --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-results"
```

## Headless Chrome WebGPU

Chrome 157 on Apple Metal 3 passed the full Material suite with 67 steps
and 3,313 checks. M63–M68 account for 594 checks.

M63–M68 check docked and floating geometry, both palettes, filled toggle
paint, named controls, roving focus and modal overflow. They exercise
wheel hide, core-only collapse, FAB-only collapse, reverse reveal and
focus protection with real browser input. The tests compare wheel
movement against the rendered row positions and exported scroll offset.
The compact profile checks dark RTL, 200% text and the scrollable
settings area beside the floating toolbar. The 320 dp horizontal
pair keeps each 48 dp action and the adjacent 56 dp FAB within the
viewport, with 8 dp between the bar and FAB.

[Browser report](web/browser-material-toolbar.json) ·
[Compact horizontal pair](web/material-toolbar-compact-dark-horizontal-fab.png)

```sh
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build material-gallery --release --no-compress
cap "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --web-root "$CARGO_TARGET_DIR/web/release" \
    --artifacts "$scratch_dir/web-results" material
```

The toolbar has named group semantics. Toolbar-specific accessibility
roles, system touch-exploration detection, kinetic settling and visibility
animation remain open. Physical touch and live VoiceOver speech still
need platform testing.
