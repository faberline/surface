# Material Directional Navigation Drawer

The modal drawer opens at the logical start edge.
It is up to 360 dp wide and has a fixed Close header
above its scrollable destination list. It uses 24 dp vector
icons and wraps long labels. The keyboard skips the disabled
destination. Each selection or dismissal returns focus to the opener.
[AndroidX drawer tokens](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/NavigationDrawerTokens.kt)

## Verified Results

- The exact Rust workspace passed 1,377 tests with 25 ignored.
  Formatting, strict Clippy, provenance and WASM consumers passed.
- Canonical Chrome 157 passed 24,712 checks. Material ran 176 steps
  and 20,738 checks. Browser accessibility ran 66 steps and 3,974
  checks. The targeted run passed 181 checks and retained nine PNGs.
- Four Apple M1 Max Metal profiles produced 46 CLI PNGs and 34 public
  macOS accessibility reads. A 320 × 240 dp capture at 200% in dark
  RTL matched its MCP PNG byte for byte. The long Chinese label uses
  two lines at 28 dp in a 112 dp row. The 72 dp header stays fixed
  while the destination list scrolls. AppKit reports the modal and
  inner navigation landmark. This receipt covers public accessibility
  queries and actions. Live VoiceOver speech remains open.

[Rust receipt](rust-summary.log) · [Chrome receipt](chrome-summary.log) ·
[Native audit](native-audit.json) · [Representative captures](captures)

## Restore the Tested Source

Start with the restored source from the
[published Current Page in Material Navigation evidence](https://github.com/faberline/surface/tree/9b3a8c950289bc09120befff410d9a6957d3dc64/docs/evidence/md3/navigation-current).

```sh
sh restore-source.sh /absolute/path/to/restored/navigation-current/source
```

The script copies the source under `/tmp` and reverses the 17 Navigation
Current paths. These paths restore the tested directional drawer.
All 662 source files, symlink strings and executable masks matched the
original checkpoint. The script verifies the six pinned product inputs
and prints the recovered source path.
[Source receipt](source-receipt.txt) · [Product digests](product.sha256)

## Repeat the Runs

Set `CARGO_TARGET_DIR` to a dedicated directory under
`/tmp/cargo-target` and set `CARGO_BUILD_JOBS=2`. From the recovered
source, run `scripts/check.sh` and
`cap cargo xtask web e2e material web-accessibility`.

Extract [the native replay capsule](directional-drawer-native-replay.tar.gz)
and run `build-and-run.sh` with the recovered source path.
The archive contains the original runner, auditor, launcher and helper
tree, plus the 17-file private QA overlay. Use an
Accessibility-authorized macOS terminal.
