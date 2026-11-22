# Surface v0.1.10 tooltip verification

This batch starts from Surface v0.1.9
(`d57fcf47d57f8f4e6e13d55e0b0e6c7547548a0c`). From a matching source
checkout set `evidence_dir` to this folder and check both manifests.

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

The 960 × 800 light profile checks a plain Tooltip and an actionable
nonmodal Dialog above their anchors. It drives Tab, Escape and the
rich action and verifies focus return. A surrounding command stays
reachable. The 320 × 580 dark RTL profile uses 200% Chinese text at
the top and side edges. It verifies placement below the trigger and
the optional caret. Both profiles check the actual Metal paint and
PNG parity across the raw API, CLI and MCP.

[Native report](native/native-tooltip-report.json) ·
[Plain and focused](native/light-plain.png) ·
[Rich at 200%](native/dark-rtl-200-rich.png) ·
[Compact edge without caret](native/dark-rtl-200-edge-no-caret.png)

Use a Cargo target under `/tmp/cargo-target` and a command environment
with access to Metal. The replay uses Reduce Motion and opens no window.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-tooltip-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
PYTHONDONTWRITEBYTECODE=1 python3 "$evidence_dir/native/verify-tooltip-native.py" \
    --source "$source_digest" --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-results"
```

## Headless Chrome WebGPU

Chrome 157 on Apple Metal 3 passed 71 Material steps and 3,504
checks. M69–M72 account for 191 checks.

The browser sends trusted mouse, key and touch events through Chrome
DevTools. It checks delay, keeping a passive tooltip open over its
painted surface, Escape, rearming, and the description of the trigger.
The rich popup keeps the page interactive, accepts keyboard focus
and returns it on dismissal. The viewport checks cover dark RTL and
200% Chinese text. The held-touch test verifies long press suppression
and the short-tap and cancelled-contact paths.

[Browser report](web/browser-material-tooltip.json) ·
[Compact rich tooltip](web/material-tooltip-compact-dark-rich.png) ·
[Passive hover](web/material-tooltip-plain-hover.png) ·
[Touch held](web/material-tooltip-touch-held.png)

```sh
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build material-gallery --release --no-compress
cap "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --web-root "$CARGO_TARGET_DIR/web/release" \
    --artifacts "$scratch_dir/web-results" material
```

The native AccessKit tests check the live relationship and help
text before and after dismissal. Native screen-reader speech and
physical device touch still need platform testing. Tooltip transitions
remain open.
