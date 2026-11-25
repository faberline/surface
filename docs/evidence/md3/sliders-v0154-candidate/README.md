# Surface Material 3 Standard Slider v0.1.54 candidate

The 635 checked source paths form tree `a92d66c4998656b1749f322723aa4324028afdc4`.
The source patch applies to the exact Icon Button tree `c694ddfc095f5cc18e3c27b99f7d02985b49abbc`.

The workspace passed 1,421 Rust tests, provenance, fmt, Clippy and three Wasm checks.
All 16 official offscreen Metal cases passed.
The separately recorded supplemental check passed 2 tests.

Two native headless recurrences each captured four original PNGs.
The API, CLI and MCP returned the same bytes, and each image matched across both recurrences.
The native observer recorded 20 samples with zero visible windows.

Five stock macOS headless cases passed 21,459 checks using Apple Metal.
They completed 300 Material steps through M301 and 77 IME checks.
The observer recorded no page errors, visible windows or surviving owned processes.

## Original screenshots

- [Native wide-light-sliders.png](native/replay-1/wide-light-sliders.png)
- [Native wide-light-focused-slider.png](native/replay-1/wide-light-focused-slider.png)
- [Native compact-dark-rtl-200-sliders.png](native/replay-1/compact-dark-rtl-200-sliders.png)
- [Native compact-dark-rtl-200-xl.png](native/replay-1/compact-dark-rtl-200-xl.png)
- [Chrome material-standard-slider-wide-light.png](stock-macos/artifacts/material-standard-slider-wide-light.png)
- [Chrome material-standard-slider-keyboard.png](stock-macos/artifacts/material-standard-slider-keyboard.png)
- [Chrome material-standard-slider-live-drag.png](stock-macos/artifacts/material-standard-slider-live-drag.png)
- [Chrome material-standard-slider-compact-dark-rtl-200.png](stock-macos/artifacts/material-standard-slider-compact-dark-rtl-200.png)

## Receipts and replay

- [Exact candidate receipt](verified-sliders-v0154.json)
- [Native action and pixel report](native/native-slider-report.json)
- [Stock offline receipt](stock-macos/verified-sliders-offline.json)
- [Chrome behavior report](stock-macos/artifacts/material-standard-slider.json)
- [Stock process observation](stock-macos/headless-window-observation.json)

Verify this copy with `shasum -a 256 -c SHA256SUMS`.
Decompress the source patch and apply it to the recorded Icon Button tree.
Verify the reconstructed files against `source.sha256`.
Use the scripts in the full archive to replay the retained stock regression checks.

The full local archive manifest digest is `f216a1a2676335b83a5d3d24e564128ddbb7b37678c41e632889cfede21ad209`.
The sealed stock input manifest digest is `bb55899a7ad66a491adcc2d53cf8447f8338c41130d48bde56cf8757933b9875`.

Coverage is partial. Hosted Linux, additional Slider forms and live screen-reader speech have separate gates.
