# Surface v0.1.42 stock macOS headless verification

All five stock suites passed with 16,123 checks on Chrome `158.0.8127.37` and the Apple Metal 3 WebGPU adapter. The adapter was not a fallback. The runner and the log capture exited with status 0. The browser recorded zero page errors, and the suite left no owned browser or Surface processes.

| Stock suite | Steps | Checks |
| --- | ---: | ---: |
| Smoke | 19 | 34 |
| Showcase IME | 20 | 77 |
| Material Gallery | 246 | 15,842 |
| Browser bridge | 24 | 94 |
| Headless DevTools | 3 | 76 |

Both I11 cases passed with a real Chrome editor, trusted keys and synthetic preedit. Select All survived a real redraw after cancel. Cancel also restored the original text and its backward selection when Chrome discarded the selected text beneath the preedit. Trusted Right and insertion produced `台文X` while paint was still held. The test captures browser and Runtime state throughout. It does not exercise a live operating-system input method.

The stock verifier checks M246 selection colors and M247 preedit underline pixels, including 28 original Canvas PNGs. It also checks the six Scroll Field reports and byte equality for the headless `Page.screenshot` API, CLI and MCP transports. Representative frames show the [forced selection](run/material-web-contrast-selection-browser-forced-partial.png), [forced preedit underline](run/material-web-contrast-preedit-underline-preedit-partial.png), [forced wheel focus](run/material-web-contrast-wheel-focused.png), [Showcase preedit](run/browser-ime-preedit.png), and [headless DevTools PNG](run/devtools-headless-screenshot.png).

- [Exact stock report](run/verified-browser.json), [run summary](run/headless-results.txt), and [runner log](run/runner.log)
- [I11 cancel and redraw](run/browser-ime-cancel-redraw.json) and [I11 backward selection restored](run/browser-ime-cancel-selected-redraw.json)
- [M246 selection report](run/browser-material-web-contrast-selection-browser.json) and [M247 underline measurements](run/material-web-contrast-preedit-underline-measurements.json)
- [Source manifest](browser-source.sha256), [binary manifest](browser-binary-sha256.txt), and [artifact manifest](package.sha256)

## Verify the evidence

```bash
shasum -a 256 -c package.sha256
PYTHONDONTWRITEBYTECODE=1 python3 -B replay/verify-stock.py passing . run
```

The portable verifier reproduces the original report from the captured results. Every JSON report and PNG is byte-identical to the local stock run. The runner log replaces one local path with `<artifact-dir>`. The [original run manifest](original-run-evidence.sha256) records the original log digest. The source and built inputs passed verification both before and after the run.

## Build and run from the sealed source

Use macOS with Chrome, Homebrew `wasm-opt`, Python 3.12, `cap`, and the project Rust toolchain. Populate the local Cargo cache before running the offline build. Set `SURFACE_CARGO_HOME` if the cache is outside `~/.cargo`. If your release checkout contains Git metadata or extra files, create a source export containing only the 549 paths in `browser-source.sha256`.

Run the following commands with the absolute source export path in `SOURCE_EXPORT`. This uses a fresh target under `/tmp/cargo-target` and keeps all Chrome tests headless. The DevTools check runs the native Gallery offscreen.

```bash
cd replay
stage=$(mktemp -d "$(pwd -P)/stock.XXXXXXXX")
bash stock-build.sh "$SOURCE_EXPORT" "$(pwd -P)/../browser-source.sha256" "$stage"
bash stock-run.sh "$stage"
```

The portable scripts use the caller's Python and Cargo cache. The original harness identity is recorded in [harness.sha256](harness.sha256). Source manifest SHA-256 `c99ed8ff89b0cee66a84732598ac2ba652802c2e52c9122c3c5bb27f250ccd4b`. Binary manifest SHA-256 `a8d443a01b4a6251684beb5c331a124cf4203cc0e4a6850f3802a369d065fb11`.
