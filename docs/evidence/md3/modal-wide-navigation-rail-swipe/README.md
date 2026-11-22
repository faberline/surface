# Standalone modal wide rail swipe dismissal

Surface opt-in swipe dismissal starts on the open panel. The visible
panel and scrim share a fraction as the pointer moves. The position
uses the width that the layout measured at primary press. The panel
moves off the logical start edge. Left-to-right and right-to-left
layouts use mirrored directions.

A slow release uses half the panel width. At 125 dp per second or
faster, the release direction selects the endpoint. The Standard
Default Spatial spring starts with the release velocity. Cancelling
returns the rail to its open endpoint. Reduce Motion settles the
chosen endpoint immediately. The dialog keeps its focus scope and
current page until the app commits dismissal.
[AndroidX anchors](https://github.com/androidx/androidx/blob/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/WideNavigationRail.kt#L1249-L1324)
· [AndroidX fling threshold](https://github.com/androidx/androidx/blob/268d841a45644cadf438fc335c793869728449ec/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/gestures/AnchoredDraggable.kt#L1535-L1541)

Predictive back, direct vertical touch scrolling and live VoiceOver
speech remain open. Wheel and keyboard scroll the destination list.

## Native results

Apple M1 Max Metal produced 18 verified PNGs using synthetic Winit
mouse and touch events in a separate instrumented test Gallery. The
adapter feeds real Surface input routes. The renderer and screenshot
code are the standard native build. Twelve held and settled PNGs
matched through the raw protocol, CLI and MCP. The original Gallery
also built independently for the transport suite.

At 34% drag travel, the scrim alpha changes from 82 to 54 in every
profile. A cancelled drag and a slow 28% release return to open. A
slow 64% release closes and restores opener focus. AppKit exposes
the named modal and current page during drag. In the 320 × 240 dp
dark RTL view, wheel scrolling moves the list from 136 to 46 dp.
Vertical touch leaves the modal and current page intact.
[Native replay](native/README.md) · [Replay archive](native/replay.tar.gz)
· [Measurements](native/measurements.json)

| Held viewport | Full panel | Panel visible at 34% travel |
| --- | --- | --- |
| [Light LTR 704 × 512](native/light-704-held.png) | 288 dp | 190.08 dp |
| [Dark RTL 704 × 512 at 200%](native/dark-rtl-704-200-held.png) | 360 dp | 237.60 dp |
| [Dark RTL 320 × 240 at 200%](native/dark-rtl-320-200-held.png) | 320 dp | 211.20 dp |

## Browser and regression checks

Chrome WebGPU passed ten cases and 1,622 checks. The new cases cover
both gesture directions, actual scrim pixels, compact scrolling and
cancellation, reduced motion, one dismissal and focus restoration.
[Browser replay](browser/README.md)
· [RTL held frame](browser/material-modal-wide-rail-swipe-touch-rtl-held.png)
· [Compact focus](browser/material-modal-wide-rail-swipe-320x240-rtl-focused-inbox.png)

Workspace Rust passed 1,435 tests with 25 ignored. Provenance,
formatting, strict Clippy and three WASM checks passed. Twelve
offscreen tests ran on real Metal. Native and browser screenshot
and bridge tests passed 641 checks with the unmodified Gallery.
[Summary](gates.json) · [Workspace tests](workspace-tests.log)
· [Metal tests](metal-gpu.log) · [Transport checks](transport.log)

## Reproduce the source

Restore the [wide rail item motion checkpoint](https://github.com/faberline/surface/tree/38a7dcbe5f1d58676f9f66fda48bdcade8ba0ece/docs/evidence/md3/wide-navigation-rail-item-motion)
using its README. From that Surface checkout apply this batch.

```sh
swipe_evidence=/path/to/modal-wide-navigation-rail-swipe
shasum -a 256 -c "$swipe_evidence/source-before.sha256"
gzip -dc "$swipe_evidence/wide-rail-swipe.patch.gz" | git apply --whitespace=error
shasum -a 256 -c "$swipe_evidence/source.sha256"
shasum -a 256 -c "$swipe_evidence/production.sha256"
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${PWD##*/}"
export CARGO_BUILD_JOBS=2
cap cargo test --locked -p surface-app --all-features --lib --test horizontal_drag
cap cargo test --locked -p surface-material --all-features --test modal_wide_navigation_rail --test navigation_drawer --test directional_navigation_drawer
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo xtask web e2e devtools-native bridge-native bridge-native-compact bridge-web
```

An independent copy applied the same delta and matched the source tree.
See the [source paths](source-paths.txt) for the batch contents.
