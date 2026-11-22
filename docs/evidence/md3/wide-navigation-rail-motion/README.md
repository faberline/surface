# Material wide navigation rail motion

The Surface Gallery uses shared Material 3 Standard springs. The
inline rail changes its width and spacing as the page reflows. Its
collapsed width is 96 dp. Its expanded width follows its content within
220 to 360 dp. The same rendered item layout stays in use through the
moving interval, so reversing the target keeps the current geometry.

A standalone modal moves by its measured width from the logical start
edge. A modal with a retained collapsed rail grows in place. Both fade
their scrim. The app retains the current page. Dismissal releases the
modal focus scope immediately and restores opener focus. The departing
paint keeps the current scroll position. Reduce Motion settles all tracks
on the current target.
[AndroidX rail implementation](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/WideNavigationRail.kt)
· [Standard motion values](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/StandardMotionTokens.kt)

Movement of individual icons and labels between arrangements,
predictive back, swipe dismissal and live VoiceOver speech remain open.

## Native results

Apple M1 Max Metal passed seven live Gallery profiles with 94 audited
PNG files. The raw protocol, CLI and MCP returned the same settled PNG
bytes in every profile. The animation samples use 0.5× capture scale.
The 960 × 800 inline attempt reached the endpoint before its first
available animation PNG. The 704 × 512 profile captures inline width
and page reflow during the same motion.

The inline samples show 96, 276 and 288 dp on opening and 288, 174,
110 and 96 dp on closing. At 174 dp on closing, all four lines of the
selected label are visible. At 110 dp the label briefly squeezes into
a narrow column that extends to the bottom of the Gallery viewport.
The light and dark wide modals show
intermediate panel and scrim pixels. At 320 × 240 in dark RTL with 200%
text, Escape restored opener focus. During the closing paint, the
Inbox row kept its y128 dp position. Both lines of the label were
visible in the settled frame. The first reduced-motion capture after
each action reached its endpoint.
[Native replay package](native-replay.tar.gz) · [Inline measurements](native-inline.json)
· [Inline raster audit](native-inline-raster-audit.json) · [Modal measurements](native-modal.json)
· [Native replay guide](native-replay.md)

| Gallery frame | PNG |
| --- | --- |
| Light inline, partway closed at 174 dp | [Full label](native-inline-mid.png) |
| Light inline, nearing the collapsed state at 110 dp | [Narrow text](native-inline-tight.png) |
| Standalone light modal, partway open | [Moving panel](native-modal-light-mid.png) |
| Retained dark RTL modal, partway open | [Moving panel and scrim](native-retained-dark-mid.png) |
| Dark RTL 200% compact focused Inbox | [Settled frame](native-compact-focused.png) |
| Same compact modal while closing | [Retiring paint](native-compact-exit.png) |

## Browser results

Chrome 157 with Apple Metal 3 WebGPU passed six focused cases and 921
checks. At 40 ms the 175 dp rail reflowed the page. The destination
ended at the rail's lower edge. All three text lines and the selected
pill paint inside the rail. The earlier screenshot exposes the cut
before the destination column shared the same width cap.
[Before](browser/pre-fix-inline-40ms.png) · [After](browser/material-wide-rail-motion-inline-40ms.png)

The standalone RTL modal held its 288 dp panel width as its position
changed. The retained modal grew in place to 219 dp at 40 ms. Both
measured scrim pixels match alpha 39. At 320 × 240 with 200% text,
the departing panel kept its 112 dp scroll offset and opener focus
returned immediately. Chrome's accessibility tree removed the modal
as its visual layer departed. [Motion measurements](browser/browser-wide-rail-motion.json)
· [Accessibility measurements](browser/browser-wide-rail-motion-accessibility.json)
· [Focused run](browser/verified.log) · [Browser replay](browser/README.md)

## Regression checks

- Workspace formatting, provenance, strict Clippy and all WASM checks
  passed. Rust ran 1,425 passing tests with 25 ignored.
- All 13 rail geometry tests and all 12 actual Metal offscreen tests
  passed. The original crop appears in the [regression transcript](inline-before-regression.log).
- The four native DevTools and bridge suites passed 641 checks on the
  final source. They include the native PNG API, CLI, MCP, regular and
  compact native bridge, and Chrome WebGPU bridge.
  [Transport log](transport.log) · [Metal log](metal-gpu.log) · [Receipts](gates.json)

## Reproduce the exact source

Start from the verified [nonmodal rail source checkpoint](https://github.com/faberline/surface/tree/2c22421a180f27cf5d4aeaa55ffa8dc3cde7e1dc/docs/evidence/md3/wide-navigation-rail).
Apply the [public screenshot](https://github.com/faberline/surface/tree/32ba585c3a7f4b0e4116b22b874671aca99fe3b7/docs/evidence/md3/public-native-screenshot)
product delta and its native bridge companion. Apply the [modal wide rail](https://github.com/faberline/surface/tree/2b36a17f12d02949732add600404bda802e36af6/docs/evidence/md3/modal-wide-navigation-rail)
delta to the same checkout. Verify the preceding manifests, then apply the
motion delta.

```sh
screenshot_evidence=/path/to/public-native-screenshot
modal_evidence=/path/to/modal-wide-navigation-rail
motion_evidence=/path/to/wide-navigation-rail-motion
gzip -dc "$screenshot_evidence/public-screenshot.patch.gz" | git apply --whitespace=error
gzip -dc "$screenshot_evidence/bridge-comparison/native-bridge-fix.patch.gz" | git apply --whitespace=error
gzip -dc "$modal_evidence/modal-wide-rail.patch.gz" | git apply --whitespace=error
shasum -a 256 -c "$screenshot_evidence/combined-source.sha256"
shasum -a 256 -c "$modal_evidence/source.sha256"
gzip -dc "$motion_evidence/wide-rail-motion.patch.gz" | git apply --whitespace=error
shasum -a 256 -c "$motion_evidence/source.sha256"
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${PWD##*/}"
export CARGO_BUILD_JOBS=2
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo xtask web e2e material web-accessibility
```

For the live native replay, extract `native-replay.tar.gz` under `/tmp`.
In the extracted folder run `bash build-and-run.sh /path/to/restored-surface /tmp/new-native-motion-captures`.
