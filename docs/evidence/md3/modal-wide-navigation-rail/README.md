# Material modal wide navigation rail

The Surface Gallery uses the shared Material 3 modal wide navigation rail.
It can open from a standalone button or keep a collapsed 96 dp rail next
to the page. The modal overlays the page and leaves its layout in place.
The panel follows the reading direction. The named Close control stays
above a destination list that can scroll on its own.

The compact height update keeps the reference header spacing on tall
windows. On a 240 dp window it reduces the spaces above and below the
header, which gives the wrapped destination enough height to stay visible.
The initial native frame and the follow-up frame show the difference.

## Verified frames

| Frame | Evidence |
| --- | --- |
| Light LTR, wide window | [Native PNG](native-wide-light-ltr-opened.png) · [Chrome PNG](browser/material-modal-wide-rail-standalone-light-ltr.png) |
| Retained collapsed rail, light LTR | [Native PNG](native-wide-light-ltr-closed-inline.png) |
| Dark RTL, 200% text, 320 × 240 dp before the height fix | [Clipped native PNG](pre-fix-native-clipped.png) |
| Dark RTL, 200% text, 320 × 240 dp with Inbox focused | [Native PNG](native-narrow-dark-rtl-200-focused-inbox.png) · [Chrome PNG](browser/material-modal-wide-rail-ax-320-focused-inbox.png) |
| Same narrow view with Inbox current | [Native PNG](native-narrow-dark-rtl-200-retained-modal.png) |

Apple M1 Max Metal produced ten PNGs in the two profiles. On the narrow
profile, the full focused row and both lines of text fit in the viewport.
The focused label measured 216 × 80 dp. Each line had 26 raster rows with
foreground text pixels. Keyboard focus scrolled the destination viewport by
136 dp. The header and background stayed in place. The narrow PNG matched
across `Page.screenshot`, CLI and MCP byte for byte. [Native audit](native-audit.json)

Live AppKit found one named modal and one named navigation landmark in
each open profile. The Close button received initial focus. Arrow keys
skipped Archive and focused Inbox with Home still current. Enter selected
Inbox. Escape and Close restored focus to the original opener.
The closed narrow Gallery still shows an overlap from its floating add
button. Width and scrim motion, predictive back, swipe dismissal, and
live VoiceOver speech remain open.

Chrome 157 on hardware Apple Metal 3 passed 82 Material and 66
accessibility checks. The tests verified the actual 320 × 240 WebGPU
drawing buffer, the 32% scrim pixels and outside pointer dismissal.
The enlarged Inbox row fit completely within the scrolled modal.
After Enter it became the current page. The retained rail exposed
the same current page when its row was visible. Escape restored
the opener and sent one dismissal. [Focused run](browser/focused.log)
· [Material measurements](browser/browser-material-modal-wide-rail.json)
· [Chrome accessibility](browser/browser-modal-wide-rail-accessibility.json)

The workspace passed 1,415 Rust tests with 25 ignored. Provenance,
formatting, strict Clippy and the three WASM checks passed.
[Test receipt](gates.json)

The full browser accessibility suite passed 70 steps and 4,338 checks.
The standard Material run reached M180 after every earlier case had
returned successfully. At that point the backing buffer had resized,
and the layout snapshot still had the old height. The final M180 and A70
tests wait for both sizes and a settled frame. The exact earlier test
source is in `modal-before-layout-wait.patch.gz`. Both source patches
build the same Gallery and WASM packages.
[Full accessibility](browser/full-accessibility.log) · [Material prefix](browser/material-prefix.log)

A separate Chrome replay passed the 12 final Material cases and 3,065
checks. It calls the same cases with the final test source. Together with
the prefix, it covers all 179 required Material steps. The two runs
include overlapping steps. The earlier attempts also recorded one
intermittent FAB timing sample, which passed in the later standard run.
[Material replay](browser/material-tail.log) · [Earlier attempts](browser/material-earlier-attempts.log)

## Reproduce

Start with the exact earlier [public native screenshot source](https://github.com/faberline/surface/tree/32ba585c3a7f4b0e4116b22b874671aca99fe3b7/docs/evidence/md3/public-native-screenshot).
Follow its preceding Wide Rail checkpoint instructions. Apply its
`public-screenshot-before-fixture-adjustment.patch.gz` to create the
baseline for this batch. Then apply the delta below in that checkout.

```sh
evidence_dir=/absolute/path/to/modal-wide-navigation-rail
gzip -dc "$evidence_dir/modal-wide-rail.patch.gz" | git apply --whitespace=error
shasum -a 256 -c "$evidence_dir/source.sha256"
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${PWD##*/}"
export CARGO_BUILD_JOBS=2
cap sh scripts/check.sh
cap cargo xtask web e2e material web-accessibility
```

The [exact browser replay](browser/replay.md) records both WASM build
commands and the runner arguments for the normal and devtools Gallery
packages. It also describes the focused replay used for M180 and A70.

Extract the [native replay](native-replay.tar.gz) in a temporary folder.
Run its `build-and-run.sh` on macOS with Metal and permission to read
accessibility properties. The runner uses public DevTools actions for
keys and obtains each capture via the production PNG API.

[Official modal wide rail](https://developer.android.com/reference/kotlin/androidx/compose/material3/ModalWideNavigationRail.composable)
