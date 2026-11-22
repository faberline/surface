# Surface v0.1.7 bottom-sheet verification

This batch starts from Surface v0.1.6
(`d72aee04962615d6280a5f68f10aa7c3f1f87cf8`). The source manifests pin
the files used at runtime. Set `evidence_dir` to this folder and check the
manifests from a matching checkout.

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

Six tests cover the standard and modal presentations and the direct
expanded modal in two viewports. The 960 × 800 profile uses the light
theme. The 320 × 480 profile uses dark mode and 200% text. Each launches a
fresh headless Gallery.

The tests check a 640 dp maximum panel width, placement at the bottom
edge, the 48 dp handle, page and sheet focus, real paint, and modal scrim
color. Enter expands the sheet and Escape restores opener focus. Compact
tests check both the wrapped Chinese text and automatic scroll to the
last focused action below the fixed header. The compact modal tests
compare bytes and metadata from the raw protocol, CLI and MCP.

[Native report](native/native-bottom-sheet-report.json) ·
[Light standard peek](native/light-standard-peek.png) ·
[Dark compact modal](native/dark-200-narrow-modal-expanded.png) ·
[Dark compact after scroll](native/dark-200-narrow-modal-scrolled.png)

Use a Cargo target under `/tmp/cargo-target` and an environment with
Metal access. This replay has no window.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-bottom-sheet-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
PYTHONDONTWRITEBYTECODE=1 python3 "$evidence_dir/native/verify-bottom-sheet-native.py" \
    --source "$source_digest" --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-results"
```

## Headless Chrome WebGPU

Chrome 157 on Apple Metal 3 passed all 54 Material steps and 2,314
checks. The six bottom-sheet steps account for 214 of those checks.

M50–M55 verify peek and expansion, the standard page and modal focus
scope, keyboard and scrim dismissal, and 200% CJK text with wheel scroll
at 320 dp. Chrome DevTools sends trusted mouse and touch events to move
the sheet, cancel a held gesture, collapse a standard sheet, and dismiss
a modal. Reduce Motion reaches every anchor with the clock held still.

[Browser report](web/browser-material-bottom-sheet.json) ·
[Compact before scroll](web/material-bottom-sheet-compact-dark-expanded.png) ·
[Touch while held](web/material-bottom-sheet-touch-held-up.png)

```sh
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build material-gallery --release --no-compress
cap "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --web-root "$CARGO_TARGET_DIR/web/release" \
    --artifacts "$scratch_dir/web-results" material
```

Physical touch, live VoiceOver speech, a shared gesture between content
and handle, and Android predictive back remain unverified or open.
