# Surface v0.1.11 carousel verification

This batch starts from Surface v0.1.10
(`239243c35b3b311f37f531e91183aa4b70be6769`). Check out
`v0.1.11` to reproduce the release. Set `evidence_dir` to this
folder. Run the manifest checks from the source tree.

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

The 960 × 800 light Gallery checks all six arrangements. It verifies
that full-screen cards fill their track. Semantic keys move focus to
offscreen cards. Show All opens a vertical list and focus comes back
to its opener. At 320 × 580, the dark RTL profile uses 200% Chinese
text. It checks mirrored focus, visible layout controls and card paint.

Two separate replays produce eight identical PNGs and identical JSON.
Every PNG and its metadata match across `Page.screenshot`, the CLI
and the MCP tool `page_screenshot`. The replay creates no window.

[Native report](native/native-carousel-report.json) ·
[Multi-browse](native/light-multi-browse.png) ·
[Full-screen](native/light-full-screen.png) ·
[Dark RTL Hero](native/dark-rtl-200-hero-center.png) ·
[Dark RTL mixed ratios](native/dark-rtl-200-mixed.png)

Use a Cargo target under `/tmp/cargo-target` and an environment that
can use Metal.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-carousel-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
PYTHONDONTWRITEBYTECODE=1 python3 "$evidence_dir/native/verify-carousel-native.py" \
    --source "$source_digest" --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-results"
```

## Headless Chrome WebGPU

Chrome 157 on Apple Metal 3 passed 76 Material steps and 3,863
checks. The five carousel steps account for 359 checks.

M73–M77 cover the six arrangements, fitted paint, physical wheel and
drag input, touch cancellation, focus and activation. Horizontal wheel
over a card preserves the page’s vertical wheel. The keyboard reaches
offscreen cards and keeps browse focus separate from activation. The
compact profile checks dark RTL, 200% text and the Reduce Motion wheel
snap after a brief pause.

[Browser report](web/browser-material-carousel.json) ·
[Hero centered](web/material-carousel-layout-hero-center.png) ·
[Dragging](web/material-carousel-mouse-drag-held.png) ·
[Dark RTL](web/material-carousel-compact-dark-rtl.png)

```sh
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build material-gallery --release --no-compress
cap "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --web-root "$CARGO_TARGET_DIR/web/release" \
    --artifacts "$scratch_dir/web-results" material
```

The carousel masks its artwork with rectangles. Rounded descendant
masks and photo rendering remain open. Physical device touch and live
VoiceOver speech still need platform testing.

[Material 3 carousel specs](https://m3.material.io/components/carousel/specs) ·
[Official Compose guide](https://developer.android.com/develop/ui/compose/components/carousel)
