# Surface Material 3 Icon Button v0.1.53 candidate

The candidate tree is `c694ddfc095f5cc18e3c27b99f7d02985b49abbc`.
The checked source manifest covers 628 files.
Apply the patch to Chips tree `48d560ab127cc1c692a5ba0eba6fb5a8949234a8`.

The workspace passed 1,399 Rust tests, provenance, formatting, Clippy and three Wasm checks.
All 16 offscreen Metal tests ran and passed.

Two native headless replays reproduced the same three PNGs.
Page.screenshot, CLI and MCP returned the same bytes within each replay.
The observer recorded zero visible windows.

Five stock macOS headless suites passed 21,241 checks.
They reached M297 with 296 completed Material steps and 77 IME checks.
Chrome used Apple Metal. The stock observer recorded zero visible windows and no surviving test processes.

## Original captures

- [Native compact-dark-rtl-200.png](native/replay-1/compact-dark-rtl-200.png)
- [Native compact-tooltip.png](native/replay-1/compact-tooltip.png)
- [Native wide-light-icon-buttons.png](native/replay-1/wide-light-icon-buttons.png)
- [Chrome material-icon-buttons-compact-dark-rtl-200.png](stock-macos/artifacts/material-icon-buttons-compact-dark-rtl-200.png)
- [Chrome material-icon-buttons-pressed.png](stock-macos/artifacts/material-icon-buttons-pressed.png)
- [Chrome material-icon-buttons-selected-focus.png](stock-macos/artifacts/material-icon-buttons-selected-focus.png)
- [Chrome material-icon-buttons-wide-light.png](stock-macos/artifacts/material-icon-buttons-wide-light.png)

## Verification files

- [Candidate receipt](verified-icon-buttons-v0153.json)
- [Native replay 1](native/replay-1/native-gallery-report.json)
- [Native replay 2](native/replay-2/native-gallery-report.json)
- [Chrome typed result](stock-macos/artifacts/material-icon-buttons.json)
- [Stock offline receipt](stock-macos/verified-icon-buttons-offline.json)
- [Stock window observation](stock-macos/headless-window-observation.json)

Use `shasum -a 256 -c SHA256SUMS` from this directory to check the compact copy.
The bundled packer checks both the original headless archive and the compact export inputs.
The full local archive manifest has SHA-256 `22d3930d3759a3ea8019dc603ad2a336be3d62c49a9c17c82634f9307ba607ce`.
The original stock archive manifest has SHA-256 `1f051254e853b2b752a5eddd6eecc87055d19007d3d960a72ff3fe3c5bcbe2cf`.

Coverage stays partial. Hosted Linux and live screen-reader speech require separate evidence.
