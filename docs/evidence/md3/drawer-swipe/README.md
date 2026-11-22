# Material modal drawer swipe verification

Surface checkpoint for [navigation drawer #25](https://github.com/faberline/surface/issues/25).
The optional drawer opens at the logical start. The panel and proportional
scrim follow the gesture. A committed release settles the panel before it
takes modal focus. Closing restores the previous focus.

## Results

- Rust workspace. 1,392 tests passed. 25 tests were ignored. Formatting,
  strict Clippy, provenance, and WASM consumer checks passed.
- Chrome WebGPU. Material passed 20,990 checks. Browser accessibility
  passed 4,155 checks. The focused swipe journey passed 294 checks.
- Native Apple M1 Max Metal. Surface `Page.screenshot` through the CLI
  returned 60 PNGs. AppKit returned 60 public accessibility checks across
  four profiles. The audit includes 16 modal states, 24 proportional scrim
  pixel checks, and four checks for a fixed button behind the preview.
  The dark 320 × 240 dp held PNG at 200% text matches the MCP PNG byte for
  byte. The compact route used injected Winit Touch events. Physical touch
  and VoiceOver speech remain open.

This was run on macOS 26.5.2 (25F84), Chrome 157.0.8103.79, and Rust 1.96.1.
[Rust summary](rust-summary.log) · [Chrome summary](chrome-summary.log)
· [Native audit](native-audit.json) · [Capture hashes](captures.sha256)

| Gallery frame | Evidence |
| --- | --- |
| Wide light, pointer drag held | [Native PNG](native-wide-light-opening-held.png) |
| Compact dark, RTL, 200% text, drag held | [Native CLI PNG](native-compact-dark-held.png) · [Native MCP PNG](native-compact-dark-mcp.png) |
| Compact dark, RTL, 200% text, committed dialog | [Native PNG](native-compact-dark-open.png) |
| Compact dark, RTL, browser touch, drag held | [Chrome WebGPU PNG](web-compact-dark-held.png) |

## Replay exact source

[Source receipt](source-receipt.txt) · [Full binary-safe patch](source.patch.gz)
· [Source paths](source-paths.txt) · [Product inputs](product.sha256)

Use a new temporary checkout of the [published base](https://github.com/faberline/surface/commit/e9ada81387dc686b9a5257348e7e50c2410942e4).
The archive restores the complete tested source from that base. Reapplying
it and hashing the restored files produced the Git tree shown in the receipt.

```sh
evidence_dir=/absolute/path/to/drawer-swipe
replay_dir=$(mktemp -d /tmp/surface-drawer-replay.XXXXXXXX)
git clone https://github.com/faberline/surface.git "$replay_dir"
cd "$replay_dir"
git switch --detach e9ada81387dc686b9a5257348e7e50c2410942e4
gzip -dc "$evidence_dir/source.patch.gz" | git apply --whitespace=nowarn
shasum -a 256 -c "$evidence_dir/product.sha256"
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-$(basename "$replay_dir")"
export CARGO_BUILD_JOBS=2
sh scripts/check.sh
cap cargo xtask web e2e material web-accessibility
```

## Repeat a native capture

The recorded native run used a test-only overlay for scripted Winit input,
AppKit observation and the screenshot command. Apply it to a copy of the
replayed checkout. [Native overlay](native-qa.patch.gz) ·
[Overlay input hashes](native-qa-inputs.json)

The [automated replay bundle](drawer-native-replay.tar.gz) contains the
original runner, auditor and their helpers. Extract it under `/tmp`.
From the extracted bundle, pass the replayed product directory and the
path to `native-qa.patch.gz` to
`sh build-and-run.sh`. From an Accessibility-authorized Mac terminal,
the runner builds and executes the same four native journeys. The
`--prepare-only` option checks the overlay and runner inputs.

```sh
gzip -dc "$evidence_dir/native-qa.patch.gz" | git apply --whitespace=nowarn
cap cargo build --locked -p surface-material-gallery \
  --features surface-material-gallery/devtools,surface-platform/macos-accessibility \
  --example fab_menu_native -p surface-devtools --bin surface-devtools
capture_dir=$(mktemp -d /tmp/surface-drawer-capture.XXXXXXXX)
export SURFACE_DEVTOOLS_SESSION="$capture_dir/session.json"
"$CARGO_TARGET_DIR/debug/examples/fab_menu_native" 320 240
```

From a second terminal, use the same session path. In the Gallery select
the actions section and scroll to the modal drawer sample. Toggle RTL
and 200% text, then swipe from the right edge. Capture while held and
after the drawer opens. In narrow windows the controls scroll into view.

```sh
surface-devtools --session "$capture_dir/session.json" page screenshot \
  --out "$capture_dir/drawer.png"
surface-devtools --session "$capture_dir/session.json" mcp
```

Use the built binary in `$CARGO_TARGET_DIR/debug`. For MCP, list tools
and call `page_screenshot` with default sizing. Decode the returned PNG
and compare it with the CLI PNG captured in the same settled state.
