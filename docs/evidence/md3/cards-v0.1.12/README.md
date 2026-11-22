# Surface v0.1.12 card verification

This batch starts from Surface v0.1.11
(`37adcff739e3057c5d70220554fdd151e5f09e17`). Check out
`v0.1.12` to reproduce the release. Set `evidence_dir` to this
folder. Run the manifest checks from the source tree.

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

The desktop profiles exercise Elevated, Filled and Outlined cards in
light and dark. The 320 dp dark RTL profile uses 200% Chinese text.
The semantic test checks focus, the single-action card, its controlled
toggle, the disabled action and separate actions inside the passive card.
The Move button changes the order and keeps focus on the same control.

The PNG checks compare the card and page colors and the painted shadow
or outline. Raw `Page.screenshot`, the CLI and MCP return the same PNG
bytes and metadata. Two independent replays produced seven identical
PNGs and identical JSON. CoreGraphics counted zero onscreen Gallery
windows during each replay.

[Native report](native/native-card-report.json) ·
[Replay comparison](native/native-replay-comparison.json) ·
[Elevated light](native/light-elevated.png) ·
[Outlined dark](native/dark-outlined.png) ·
[Dark RTL 200%](native/dark-rtl-200.png)

Use a Cargo target under `/tmp/cargo-target` and an environment that
can use Metal.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-card-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
PYTHONDONTWRITEBYTECODE=1 python3 "$evidence_dir/native/verify-card-native.py" \
    --source "$source_digest" --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-results"
```

## Headless Chrome WebGPU

Chrome 157 passed 81 Material steps and 4,086 checks. The five card
steps account for 223 checks.

M78–M82 cover the three variants in both themes. The browser sends
trusted pointer, key, wheel and touch events through Chrome DevTools.
It checks hover, held press, focus, toggle semantics, a disabled card,
independent actions and reorder. Mouse and touch cancel restore the
card to its original position. The Move button performs the same
reorder. The compact test exercises 320 dp dark RTL with 200% text.

[Browser report](web/browser-material-card.json) ·
[Elevated light](web/material-card-elevated-light.png) ·
[Held drag](web/material-card-touch-drag.png) ·
[Dark compact action](web/material-card-compact-dark-action.png)

```sh
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build material-gallery --release --no-compress
cap "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --web-root "$CARGO_TARGET_DIR/web/release" \
    --artifacts "$scratch_dir/web-results" material
```

Animated ripple and rounded descendant clipping remain shared rendering
gaps. Physical device touch and live VoiceOver speech still need
platform testing.

[Material 3 card guidelines](https://m3.material.io/components/cards/guidelines) ·
[Official Compose guide](https://developer.android.com/develop/ui/compose/components/card)
