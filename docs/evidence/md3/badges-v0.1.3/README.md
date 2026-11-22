# Surface v0.1.3 Badge verification

The Badge supports a standalone or attached dot, short label or count.
The anchor keeps its controlled selection, focus and actions. The
clearance option follows its owner and respects the visible clip.

The starting Surface revision was `d508799747252376a8222fba87b427a044619ba0`.
The [product manifest](product-source.sha256) records 288 candidate
source paths. Its SHA-256 is
`fa06c7a2c00695689ea81953a8ff515881315ef28dfa1ee5a8b0ca745560d076`.
The separate [browser manifest](browser/badge-browser-product-source.sha256)
pins the 274 browser build and test inputs.

## Chrome WebGPU

Chrome 157.0.8103.79 used the Apple Metal 3 WebGPU adapter.
All 34 Material steps passed, covering 605 checks. Steps M32 through M35
check both badge sizes, hidden paint, the full accessible count, focus,
pointer and keyboard input, selection, clipping and RTL geometry.
[Browser log](browser/badge-browser-final.log) · [Browser receipt](browser/browser-material-badge.json)

At 320 dp with RTL and 200% text, the initial candidate painted the
`999+` badge over the Inbox icon. The first pixel check found zero visible icon pixels. The next
layout exposed part of the icon. The stronger check still failed
because the count overlapped 14 dp of the icon slot. The final
candidate overlaps 4 dp. Chrome measured 602 Inbox icon pixels, equal to
the independently rendered contour in that profile.
[Initial failure](browser/previous-artwork-overlap/result.log) ·
[14 dp failure](browser/previous-artwork-overlap/14dp-check.log) ·
[Final compact screenshot](browser/material-badge-dark-rtl-nav-overflow-200.png)

- [Standalone badges](browser/material-badge-light-standalone.png)
- [Attached count](browser/material-badge-light-ltr-count.png)
- [Compact RTL with a localized label](browser/material-badge-dark-rtl-label-200.png)

## Native macOS and Metal

The stock Gallery and a size-only launcher passed light and dark
960 × 800 and dark 320 × 480 RTL at 200% text. The driver used the
production `Page.screenshot` endpoint and the regular CLI and MCP
clients. The compact PNG bytes matched across all three paths.
The Inbox retained 144 of 157 reference contour pixels in the native
compact capture.

A single semantic Enter advanced the count from 999 to 1000. The
paint showed `999+`. AppKit retained focus and exposed the full count
with its press action. Tab reached the following navigation control.
The driver also checked the CJK glyph and the owner-clipped badge.
Physical keyboard, physical touch and live VoiceOver speech remain open.
[Native replay and screenshots](native/replay/README.md)

The native report keeps its build manifest ID. The only later change
inside that manifest was a Rust test file. It does not build into the
Gallery or DevTools binaries. The release manifest captures the final
code and tests together.
[Native build manifest](native/native-runtime-source.sha256)

## Workspace and GPU checks

The workspace passed format, provenance, strict Clippy, 694 Rust tests
and all three WASM checks. Six tests were ignored in the workspace run.
Five offscreen regression tests passed separately on Apple M1 Max Metal.
[Workspace log](workspace.log) · [Metal log](metal.log)

## Replay

Use a disposable checkout from the candidate or its verified merge,
with Rust first in `PATH` and the repo’s WebAssembly tools available.

```sh
shasum -a 256 -c "$evidence_dir/product-source.sha256"
shasum -a 256 -c "$evidence_dir/browser/badge-browser-product-source.sha256"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${PWD##*/}"
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo xtask web build material-gallery --release --no-compress
cap cargo build --locked -p surface-e2e --bin surface-e2e
"$CARGO_TARGET_DIR/debug/surface-e2e" material \
  --web-root "$CARGO_TARGET_DIR/web/release" --artifacts "$browser_output"
```

Set `evidence_dir` to this evidence directory and `browser_output` to a
new temporary directory. The Metal check needs a real non-fallback
adapter.
