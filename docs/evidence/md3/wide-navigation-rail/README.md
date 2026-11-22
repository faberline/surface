# Material nonmodal wide navigation rail verification

Follow-up evidence for [Surface navigation rail #26](https://github.com/faberline/surface/issues/26).
The nonmodal rail is 96 dp collapsed and fits content between
220 and 360 dp expanded. It reflows the adjacent page. The Gallery
switches to a bottom bar below 600 dp. Stable destination IDs keep
focus and the current page across the change.

The expanded label uses the content width limit during intrinsic
measurement. At 200% text the full label, row and rail grow together.
The fixed-width collapsed layout measures its own label at the
available width and keeps its natural row stack.

## Results

- Rust. 1,397 tests passed. 25 tests were ignored. Formatting,
  provenance, strict Clippy, and three WASM consumer checks passed.
- Apple M1 Max Metal. 18 Surface CLI screenshots and 18 public
  AppKit reads passed. Both palettes worked at 320 × 240 dp. The
  dark compact CLI and MCP PNGs match byte for byte. The run
  verified disclosure, focus, current page, resize at 599/600 dp,
  containment of the RTL 200% label, and collapsed stack height.
- Chrome WebGPU. M179 passed 450 focused checks and A69 passed
  135 focused accessibility checks on Apple metal-3. The focused
  run verified the actual 320 × 240 dp canvas. The full Material and
  accessibility suites passed 25,734 checks. The inherited FAB motion
  fixture samples 144 ms to observe its partially painted label.

[Rust receipt](workspace-receipt.json) · [Native audit](native-audit.json)
· [Chrome focused summary](chrome-focused-summary.log) · [Chrome full summary](chrome-full-summary.log)
· [Capture hashes](captures.sha256)

| Gallery frame | Evidence |
| --- | --- |
| Light LTR expanded | [Native PNG](native-wide-light-expanded.png) |
| Dark RTL expanded, 200% | [Native PNG](native-wide-dark-expanded.png) · [Chrome PNG](web-wide-dark-expanded.png) |
| Dark RTL collapsed, 200% | [Native PNG](native-wide-dark-collapsed.png) |
| Dark RTL bottom bar, 320 × 240 dp | [Native CLI PNG](native-compact-dark.png) · [Native MCP PNG](native-compact-dark-mcp.png) · [Chrome PNG](web-compact-dark.png) |

The native run used macOS 27.2.1 (26B131). The browser was
Chrome 157.0.8103.79. The nonmodal motion, modal wide rail and
live VoiceOver speech remain open.

## Reproduce the exact source

Restore the [drawer swipe source](https://github.com/faberline/surface/tree/3d817301c8905043f06ca7e0533833608f99ecc5/docs/evidence/md3/drawer-swipe)
in a new temporary checkout. Apply only its product source
archive. From that checkout apply this batch's delta. It was
replayed and compared byte for byte with the staged product.

[Delta](wide-rail.patch.gz) · [Delta replay receipt](source-receipt.txt)
· [Product hashes](product.sha256) · [Browser fixture hashes](browser-fixtures.sha256)

```sh
evidence_dir=/absolute/path/to/wide-navigation-rail
gzip -dc "$evidence_dir/wide-rail.patch.gz" | git apply --whitespace=error
shasum -a 256 -c "$evidence_dir/product.sha256"
shasum -a 256 -c "$evidence_dir/browser-fixtures.sha256"
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${PWD##*/}"
export CARGO_BUILD_JOBS=2
sh scripts/check.sh
cap cargo xtask web e2e material web-accessibility
```

The [native replay](native-replay.tar.gz) contains the exact native QA
overlay, runner, and auditor. Extract it into a new directory under
`/tmp`. From an Accessibility-authorized Mac terminal run
`sh build-and-run.sh /absolute/path/to/verified/surface`. It builds
in a separate temporary copy and captures both CLI and MCP frames.

[Material wide rail guidance](https://developer.android.com/reference/kotlin/androidx/compose/material3/WideNavigationRail.composable)
· [Pinned AndroidX source](https://github.com/androidx/androidx/blob/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/WideNavigationRail.kt)
