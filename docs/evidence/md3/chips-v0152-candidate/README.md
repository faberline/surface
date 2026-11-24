# Surface Material Chips v0.1.52 candidate

Source tree `48d560ab127cc1c692a5ba0eba6fb5a8949234a8`.
The source patch applies to the exact Tabs v0.1.51 tree.
The archive records 624 checked source paths.

The workspace passed 1,390 Rust tests, provenance, formatting, Clippy, and
WebAssembly checks. All 16 offscreen Metal cases ran and passed. The native
AccessKit fixture checks separate actions and rejects stale targets.

The native Gallery ran twice in headless mode. Within each run,
Page.screenshot, CLI, and MCP produced the same PNG bytes.
The images also match across the two runs. The profiles cover wide light
and dark RTL at 320 dp with 200 percent text.

The five stock macOS headless suites passed 20,949 checks.
The suite ran 292 Material steps through M293. Chrome used Apple Metal.
The observer recorded zero visible windows and zero surviving test
processes. The report includes earlier Material and input regressions.

[Browser result](stock-macos/verified-chips-offline.json) ·
[Process observation](stock-macos/headless-window-observation.json) ·
[Native replay](native/replay-1/native-gallery-report.json)

- [compact dark rtl 200](stock-macos/artifacts/material-chips-compact-dark-rtl-200.png)
- [wide focus](stock-macos/artifacts/material-chips-wide-focus.png)
- [wide light](stock-macos/artifacts/material-chips-wide-light.png)
- [wide pressed](stock-macos/artifacts/material-chips-wide-pressed.png)

## Rerun

Apply the decompressed patch to the pinned Tabs tree and check `source.sha256`.
Use the [original runbook](original-runbook.md) for the local proof layout.
With the repo's Rust build settings, use `cap sh scripts/check.sh`,
`cap sh scripts/check-gpu.sh`, and the native scripts and browser observer
in this directory. Use `--headless` for the native Gallery and
`--headless=new` for Chrome. Decompress the retained validator patch
before running the original observer.

This compact copy retains the original receipts and representative images.
The full local proof manifest digest is `f18f0c121da62e2b01954caf7c07eada0d69917a5c5d917859292f0c9ad63fcc`.
The full stock run manifest digest is `fd01882d79e39a72fb4904dc944627bade994f2b96177da361c731ae79ab0b74`.
`SHA256SUMS` covers the files in this GitHub copy.

Coverage remains partial. Linux CI, live screen-reader speech, system high
contrast, and optional drag reordering are separate checks.
