# Surface v0.1.42 stock macOS browser verification

The tested candidate is `ab8615c8495d99a652f4fac38d4db37f08c82c7f`. The stock E2E binary and Gallery ran in Chrome/158.0.8127.37 with an Apple Metal 3 WebGPU adapter. The adapter reports hardware rendering. All five headless suites passed 16,103 checks. Material passed 246 steps and 15,842 checks. All 40 reports that record page errors were empty. No process owned by the run remained. Showcase passed all 20 input-method steps, including line endings and Unicode offsets. The native DevTools case returned the same PNG through the API, CLI, and MCP.

[Case totals](headless-results.txt) · [IME line ending test](reports/browser-ime-line-endings.json) · [Strict verification report](reports/verified-browser.json) · [Stock runner log](runner.log) · [Build log](build.log) · [Exact input hashes](stock-inputs.json)

## Contrast and text paint

- M241 accepted the independently measured Canvas, Button, and Highlight pairs. M242 measured a live Normal → Forced → Normal change on the actual Canvas.
- M243 kept an invalid draft, focus, and synthetic composition through palette changes. M244 and M245 checked the popup and wheel outlines. The six previous Scroll Field reports also passed.
- M246 checked the actual selection fill and glyphs at 1.25 device pixel ratio. M247 checked the 1 dp preedit underline at 2 device pixel ratio. The held native ArrowRight also survives a real forced color redraw before keyup. A separate, scoped event-delay check verifies ownership before keyup, and the backwards range retains its direction. The same underline switches to HighlightText on all 92 sampled pixels inside the selection. Both flanks keep CanvasText. Normal editor pixels restore exactly.

[Contrast report](reports/browser-material-web-contrast.json) · [Caret and redraw report](reports/material-web-contrast-selection-browser-native-arrow.json) · [M247 report](reports/browser-material-web-contrast-preedit-underline.json) · [Actual M247 pixel samples](reports/material-web-contrast-preedit-underline-measurements.json)

| Capture | Image |
| --- | --- |
| Popup Normal | [PNG](captures/material-web-contrast-popup-normal.png) |
| Popup Forced | [PNG](captures/material-web-contrast-popup-forced.png) |
| Popup restored Normal | [PNG](captures/material-web-contrast-popup-restored.png) |
| Wheel boundary | [PNG](captures/material-web-contrast-wheel-resting.png) |
| Fractional DPR selection | [PNG](captures/material-web-contrast-selection-browser-forced-partial.png) |
| Selected preedit underline | [PNG](captures/material-web-contrast-preedit-underline-preedit-partial.png) |

All 28 contrast Canvas PNGs and their raw JSON reports preserve the captured bytes. The Chrome preference changes and input composition use CDP emulation. Actual system settings changes, operating-system input, and live speech need separate verification.

## Earlier candidates

The first stock run exposed a cursor rollback. The [old candidate trace](diagnostics/old-candidate-native-arrow.json) records the trusted arrow advancing to 4 and repaint returning it to 3. [The old Canvas](diagnostics/old-candidate-native-arrow-forced.png) confirms that the paint really changed. The first repair also exposed a selection loss immediately after composition committed 中文. The [IME trace](diagnostics/first-fix-ime-selection-trace.json) records that separate failure. The [isolated comparison](diagnostics/isolated-red-green-comparison.json) gives the exact old and fixed cursor offsets, the outcome of 100 repeated keyboard trials on each build, and the focused receipt hashes. This page’s stock results use the corrected committed candidate.

## Reproduce

Use macOS, Chrome at its usual application path, an Apple Metal 3 adapter, the Command Line Tools SDK, `cap`, Python 3.12, wasm-bindgen CLI, and wasm-opt. Prepare a local Cargo cache for an offline build. Scripts default to `/tmp/surface-md3-cargo` when `CARGO_HOME` has no value.

Run from this evidence folder. Use an exact checkout of the candidate. The output and target stay in `/tmp`.

```sh
web_evidence="$(pwd -P)"
contrast_source=/path/to/surface-at-ab8615c
contrast_output="$(mktemp -d /tmp/surface-contrast-web-stock.XXXXXXXX)"
shasum -a 256 -c SHA256SUMS
bash "$web_evidence/web-scripts/stock-build.sh" \
  "$contrast_source" ab8615c8495d99a652f4fac38d4db37f08c82c7f \
  "$web_evidence/stock-source.sha256" "$contrast_output"
bash "$web_evidence/web-scripts/stock-run.sh" "$contrast_output"
```

The first step checks all files against the package hashes. The build checks the [candidate source](stock-source.sha256) and pins the actual [build inputs](stock-build-inputs.sha256) and [binary and Web bundle files](stock-binaries.sha256). The archived binary hashes identify the original build. The new replay seals its own output files before running. Both replays use only stock binaries and the five explicitly selected headless suites. The portable wrapper uses a general Python path and temporary directory. [The original harness hashes](original-harness.sha256) identify the scripts used for the archived run.

Only host paths changed in the archived logs. [The input record](stock-inputs.json) records the original raw log hashes.
