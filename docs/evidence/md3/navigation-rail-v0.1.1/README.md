# Navigation Rail verification for Surface 0.1.1

This packet covers the [Navigation Rail PR](https://github.com/faberline/surface/pull/174)
at source commit [b583560](https://github.com/faberline/surface/commit/b58356033226e815c08566bdd1ed0dcf5589ee57).
The local package was built from this work using the
[production source hashes](product.sha256). The PR proposes a source
release of Surface.

The Gallery shows the classic vector rail, adaptive inline wide rail,
retained modal, and standalone modal with swipe dismissal. The shared
changes keep the original glyph APIs and the existing Style fields.
The rail uses named destinations, current-page state, logical
directions, reduced motion and a compact bottom bar.

## Native

Apple Metal drew 21 verified PNGs across six profiles. Actual AppKit
queries confirmed the modal scope, current page, and focus during the
gesture. Cancel returned to open. Dismissal restored opener focus.
The compact dark RTL case used 200% text at 320 × 240 dp. Both lines
of Inbox rendered. Wheel scrolling moved the list while the header stayed.
[Modal measurements](native/modal-measurements.json)
· [Classic and adaptive measurements](native/inline-measurements.json)

A separate test harness supplied synthetic Winit input and read the
rendered pixels through a test-only Page.screenshot adapter. The archive
contains those overlay sources and the reproducible builds. The stock
release carries the rail; the screenshot endpoint is tracked separately.
[Native replay](native/replay.tar.gz) · [Replay guide](native/README.md)

| Native Gallery frame | PNG |
| --- | --- |
| Classic vector rail in light theme | [PNG](native/classic-light-704.png) |
| Light standalone modal held during a swipe | [PNG](native/light-704-held.png) |
| Dark RTL at 200% held during a swipe | [PNG](native/dark-rtl-704-200-held.png) |
| Dark RTL 320 × 240 with Inbox focus at 200% | [PNG](native/dark-rtl-320-200-open.png) |

## Chrome WebGPU

Chrome 157 passed all 26 Material steps and 328 checks. They include
the original Material tests and four new rail steps. The inline
rail reaches 175 dp at 40 ms and switches to a bottom bar in a
320 × 240 window. Both mouse and touch gestures moved the 286 dp
modal by 82.94 dp in the correct physical direction. The WebGPU
scrim pixels match the expected colors. The compact case scrolls
until both Inbox lines are visible at 200%. Chrome keeps the
named modal through a drag and restores opener focus after release.
[Transcript](browser/verified.log) · [Measurements](browser/measurements.json)
· [Compact PNG](browser/material-navigation-rail-modal-rtl-320-focused-inbox.png)

## Local engineering checks

Workspace Rust passed 645 tests with 6 ignored. Provenance,
formatting, strict Clippy and three WASM checks passed. Five
offscreen tests ran successfully on actual Apple Metal.
[Gate summary](gates.json) · [Workspace results](workspace-tests.log)
· [Metal regression results](metal-regressions.log)

## Reproduce

Use the PR source and Cap. The native archive contains a dedicated
replay for macOS and Apple Metal. Run the browser journey with Chrome
and WebGPU.

```sh
rail_evidence=/path/to/navigation-rail-v0.1.1
shasum -a 256 -c "$rail_evidence/product.sha256"
shasum -a 256 -c "$rail_evidence/browser/fixtures.sha256"
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${PWD##*/}"
export CARGO_BUILD_JOBS=2
cap sh scripts/check.sh
cap cargo xtask web e2e material
```

Predictive back, direct vertical touch scrolling and live VoiceOver
speech remain open.
