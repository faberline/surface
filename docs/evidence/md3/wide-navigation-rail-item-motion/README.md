# Wide navigation rail destination motion

Inline and retained modal destinations use the same moving item.
The icon and selected indicator move together. A long label keeps
its compact wrap under the icon near the 96 dp endpoint. Its opacity
reaches zero at the arrangement change. The label then reappears at
the logical end of the icon. The app keeps one destination ID, one
button and one action. Both endpoints use the existing static views.

The label fade follows the pinned AndroidX curve. Surface also keeps
the compact row height through the first half of the transition, so the
moving icon cannot cover the text. The row then moves toward its expanded
height. Reduce Motion snaps to the requested endpoint.
[AndroidX item transition](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/NavigationItem.kt#1183)

## Native results

Apple M1 Max Metal produced 44 audited PNGs. The 118 dp closing frames
show both compact lines in light LTR and all five lines at 200% in dark
RTL. The four settled PNGs match byte for byte through the raw protocol,
CLI and MCP. AppKit exposes one current Inbox button. Keyboard navigation
passes disabled Archive and selects Inbox.
[Native replay](native/README.md) · [Replay archive](native/replay.tar.gz)
· [Pixel measurements](native/item-raster-audit.json)

| Surface Gallery frame | PNG |
| --- | --- |
| Light LTR closing at 118 dp | [Two complete lines](native/light-collapse-118dp.png) |
| Dark RTL closing at 118 dp with 200% text | [Five complete lines](native/dark-rtl-200-collapse-118dp.png) |
| Dark RTL 320 × 240 retained modal with Inbox focus | [Two complete lines](native/compact-rtl-focused.png) |

Escape restores opener focus in the 320 × 240 dp retained modal.
The panel remains at a 136 dp scroll offset during closing paint
while the items change height. A closing PNG shows the panel at
160 dp. The first available native opening PNG shows the endpoint.
[Retained modal measurements](native/retained-measurements.json)

## Chrome WebGPU results

Eight cases passed 1,374 checks. The four controlled early widths
show complete compact wrapping in both directions at 200% text.
The item fades through the arrangement change while its icon
and selection indicator move. A real Chrome Enter at 119 dp
selects Inbox once and keeps focus there.
[Browser replay](browser/README.md) · [Light LTR at 128 dp](browser/material-wide-rail-motion-item-ltr-22ms.png)
· [Dark RTL at 128 dp](browser/material-wide-rail-motion-item-rtl-22ms.png)

## Regression checks

Workspace provenance, formatting, strict Clippy and all three
WASM checks passed. Rust ran 1,430 passing tests with 25 ignored.
All 12 real Metal offscreen tests passed. The four screenshot and
bridge suites passed 641 checks, including native API, CLI and MCP.
[Gate summary](gates.json) · [Rust results](workspace-tests.log)
· [Metal results](metal-gpu.log) · [Transport results](transport.log)

## Reproduce the source

Restore the [preceding wide rail motion checkpoint](https://github.com/faberline/surface/tree/ba936c54a75a747797b2927cf9489e1b71883e92/docs/evidence/md3/wide-navigation-rail-motion)
with its README. Then apply this batch from the restored Surface checkout.

```sh
item_motion_evidence=/path/to/wide-navigation-rail-item-motion
shasum -a 256 -c "$item_motion_evidence/source-before.sha256"
gzip -dc "$item_motion_evidence/rail-item-motion.patch.gz" | git apply --whitespace=error
shasum -a 256 -c "$item_motion_evidence/source.sha256"
shasum -a 256 -c "$item_motion_evidence/production.sha256"
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${PWD##*/}"
export CARGO_BUILD_JOBS=2
cap cargo test --locked -p surface --all-features --test measured_group
cap cargo test --locked -p surface-material --all-features --test wide_navigation_rail --test modal_wide_navigation_rail
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo xtask web e2e material web-accessibility
cap cargo xtask web e2e devtools-native bridge-native bridge-native-compact bridge-web
```

An independent checkout applied the same delta, then matched all
Surface source files against the verified working copy.
The [source inventory](source-paths.txt) lists the paths changed here.

Predictive back, swipe dismissal and live VoiceOver speech remain open.
