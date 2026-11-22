# Native Surface screenshot verification

The public `Page.screenshot` method captures the drawing commands from the
last presented native frame. Build the app with `devtools` and start it with
`SURFACE_DEVTOOLS_SESSION` set to a fresh file. The app advertises the method
through `Devtools.capabilities`. [Issue #155](https://github.com/faberline/surface/issues/155)

## Use from an agent

```sh
surface-devtools --session /path/to/session.json page screenshot --max-side 1200 --out surface.png
surface-devtools --session /path/to/session.json mcp
```

The CLI creates a private PNG. It returns an error if the destination exists.
Omit `--out` to get JSON with the PNG in `data_base64`. For MCP, use the second
command as a stdio server and call `page_screenshot` with optional `max_side`
and `scale`. The first content item is an `image/png` block. The second item
gives width, height, and effective scale as text.

The raw protocol takes the same parameters. The caller reads the endpoint and
token from the private session descriptor. Requests and replies use one JSON
object per line.

```json
{"v":1,"id":1,"method":"Page.screenshot","params":{"max_side":1200},"token":"<session token>"}
```

## Results

- Rust. 1,409 tests passed. 25 were ignored. The workspace formatting,
  strict Clippy, provenance, and all three WASM checks passed. The
  feature-off dependency tree for `surface-platform` excludes the screenshot
  protocol and PNG encoder. [Workspace and Metal receipts](gates.json)
- Metal. The 12 offscreen GPU tests passed. The real native DevTools
  suite passed all 27 steps and 353 checks, including raw protocol,
  CLI and MCP. The regular and compact native Jet bridge and the
  Chrome WebGPU bridge also passed. The four suites ran 641 checks.
  [Transport transcript](transport.log) · [Offscreen GPU PNG](metal-offscreen-capture.png)
- The native bridge had an inherited short-write bug on macOS. It
  affected the preceding Wide Rail version as well. The companion
  `native-bridge-fix.patch.gz` sets blocking mode on each accepted
  socket so the existing write deadline can take effect. [Diagnosis and fix](bridge-comparison/README.md)
- The native Gallery replay used Apple M1 Max on macOS 27.2.1 (26B131).
  Raw protocol and CLI returned the same PNG bytes in all three profiles.
  At 320 × 240 dp, CLI and MCP also matched byte for byte. The AppKit
  read confirmed one named navigation landmark per window and focus on
  the current Inbox destination. [Native audit](native-audit.json)
- Native fault runs caught swapped red and blue, a stale image after a
  theme change, and a request for 400 px that returned 960 px. With fault
  injection disabled, the same runner passed 128 checks. [Fault results](negative-controls/receipt.md)

| Gallery frame | Evidence |
| --- | --- |
| Light LTR expanded | [Native PNG](light-expanded.png) |
| Dark RTL expanded, 200% text | [Native PNG](dark-rtl-200.png) |
| Dark RTL bottom bar, 200% text, 320 × 240 dp | [Native CLI PNG](compact-dark-rtl-200.png) · [Native MCP PNG](compact-dark-rtl-200-mcp.png) |
| Native transport test before and after changing theme | [Light PNG](transport-native-light.png) · [Dark PNG](transport-native-dark.png) |
| Chrome bridge journey | [Chrome PNG](bridge-web.png) |

The narrow native Gallery frame shows a small label overlap from the
fixed floating add button. The AppKit probe reads accessibility properties.
Live VoiceOver speech remains open.

## Reproduce the source and checks

Restore the exact product source of the preceding [nonmodal wide rail checkpoint](https://github.com/faberline/surface/tree/2c22421a180f27cf5d4aeaa55ffa8dc3cde7e1dc/docs/evidence/md3/wide-navigation-rail)
in a new checkout under `/tmp`. Apply the delta from this directory. The
published delta was applied to an independent copy of that checkpoint and
compared with the source used in the final workspace run.

```sh
evidence_dir=/absolute/path/to/public-native-screenshot
gzip -dc "$evidence_dir/public-screenshot.patch.gz" | git apply --whitespace=error
gzip -dc "$evidence_dir/bridge-comparison/native-bridge-fix.patch.gz" | git apply --whitespace=error
shasum -a 256 -c "$evidence_dir/combined-source.sha256"
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${PWD##*/}"
export CARGO_BUILD_JOBS=2
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo tree --locked -p surface-platform --no-default-features -e normal
cap cargo xtask web e2e devtools-native bridge-native bridge-native-compact bridge-web
```

The AppKit and visible Metal [replay packet](native-replay.tar.gz) uses a
small launcher that chooses the starting window size. It uses the public
protocol for input and capture. Extract the packet under `/tmp` and run
`bash build-and-run.sh /path/to/verified-surface-checkout /tmp/new-surface-captures`.
It pins the [source before the DOM fixture adjustment](public-screenshot-before-fixture-adjustment.patch.gz).
Apply that delta to a fresh copy of the earlier Wide Rail checkpoint and
verify `source-before-fixture-adjustment.sha256` to reproduce the exact
Gallery and seeded fault runs. Compared with this earlier delta, the
final screenshot delta also updates the E2E assertions for the documented
DOM schema versions. Both deltas use the same screenshot renderer,
protocol, and client source.
