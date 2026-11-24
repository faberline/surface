# Surface Material 3 Radio Button v0.1.50 candidate

The candidate adds named groups and controlled radio options. The
application owns the selected value. The label and radio share one
target. The earlier Radio builders stay available.

Tab enters at the checked option when it is enabled. Without an enabled
checked option, Tab enters at the first enabled choice and Shift+Tab
enters at the last. Arrows wrap, skip disabled choices and request
selection. In RTL, Left and Right mirror. A checked option remains
focusable and Space leaves its state unchanged.

The Radio uses a 20 dp ring, a 10 dp dot, a 40 dp state layer
and a 30 dp exterior focus ring. The dot uses retained motion. Reduce
Motion applies the correct endpoint immediately.
[Material 3 Radio button specification](https://m3.material.io/components/radio-button/specs)

## Exact source

The candidate has 607 source files. Its manifest SHA-256 is
`a031c8b5dc2700f69cb131182842722976d57726dd01c2e0a2d99e643e406544`.
The [Radio patch](inputs/surface-v0.1.50.patch.gz) has uncompressed
SHA-256
`137d2d62e427f2ee8d0032b35ac57a9950d74e251675ac45e06594b2b1f6e9b1`.
Apply it to Checkbox v0.1.49 with sealed source digest
`3a9aeda30a8ca3520afd04d9d4db8c26f3a900dc5aae20fad14e4d366d2544fe`.
[Checkbox evidence and source patch](https://github.com/faberline/surface/tree/f660063aa1ff172adfd30ab9f8d440fd512b1c2d/docs/evidence/md3/checkbox-v0149-candidate)

Decompress the patch with `gzip -dc`. Verify the resulting source files
from the checkout with `shasum -a 256 --status -c` and
[the pinned manifest](inputs/source.sha256).

## Native replays

Two headless replays matched the checked-first entry, first/last entry
when clear, arrow wrapping, disabled rejection, focus restoration and
CJK label wrapping. The light profile is 960 × 800. The dark RTL
profile is 320 × 720 with 200% text. Each PNG matched across
`Page.screenshot`, CLI and MCP. Corresponding replay PNGs are identical.
All twelve window observations were zero.
[Light](native/exact-1/wide-light.png) · [Dark RTL](native/exact-1/compact-dark-rtl-200.png) · [First receipt](native/exact-1/native-gallery-report.json) · [Second receipt](native/exact-2/native-gallery-report.json)

Use the [offline verifier](native/scripts/verify-native-replays.py)
with Python and Pillow. Run the replay script for a fresh offscreen
capture on macOS with a Metal adapter. Set a temporary
`CARGO_TARGET_DIR` for the exact checkout. Set `scripts` to the
evidence package's `native/scripts` path and `proof_dir` to a fresh
folder under `/tmp`.

```sh
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
  --features surface-material-gallery/devtools --bins
cap python3 -B "$scripts/replay-radio.py" \
  --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
  --cli "$CARGO_TARGET_DIR/debug/surface-devtools" \
  --out "$proof_dir/native"
```

## Workspace and Chrome checks

The workspace gate passed 1,349 Rust tests, formatting, Clippy, source
provenance and WebAssembly checks. All 16 offscreen Metal tests passed.
[Workspace log](workspace-exact.log) · [GPU log](gpu/gpu-exact.log)

All five macOS headless suites passed, with 20,293 checks and 284
Material steps. Chrome WebGPU used Apple Metal. The four Radio steps
verify real accessibility roles, selection, Tab in both directions,
mirrored RTL arrows, press and cancellation, and actual paint.
The window monitor covered primary Chrome, detached bridge Chrome and
the native Gallery. It observed zero visible windows and no survivors.
[Runner log](stock-mac/runner.log) · [Window check](stock-mac/headless-window-observation.json) · [Independent receipt](stock-mac/verified-radio-offline.json)

[Wide keyboard focus](stock-mac/artifacts/material-radio-wide-focused.png) · [Horizontal layout](stock-mac/artifacts/material-radio-wide-horizontal.png) · [Held](stock-mac/artifacts/material-radio-wide-held.png) · [Dark RTL](stock-mac/artifacts/material-radio-compact-dark-rtl-200.png)

Rerun from the exact checkout using the five headless suite names.

```sh
cap cargo xtask web e2e smoke ime material bridge-web devtools-headless \
  --artifacts /tmp/surface-radio-browser-evidence
```

Use the [independent browser verifier](stock-mac/verify_radio_report.py)
with `--profile stock-macos`, the runner log, artifacts and source seal.
The complete local archive has 745 files and manifest digest
`1abf8ba9dc18045cfb1300b43276c63a2f6d7925cb89b150182b3d27e0e29120`.

## Primary sources

The Google page and DSDB tokens use the November 19 export. The page
SHA-256 is
`412eed3c3dfc7a7479209f6dba63a34e85096a9cfeb30e88e858b3e8aa1e67ce`.
The token SHA-256 is
`1037ed81731b05681500af73c15170bbce72c26f5c96a65743dde75188a2d044`.
[Google page](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/091d4b7c-1ab2-4050-963a-4e2558ee5d08.json) · [Google token export](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.6acb269a58d5f93f.json) · [WAI Radio Group pattern](https://www.w3.org/WAI/ARIA/apg/patterns/radio/)
