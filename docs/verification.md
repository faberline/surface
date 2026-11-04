# Framework verification

## Material controls and current DOM export

Recorded on 2026-11-04, on macOS / Apple M1 Max, Rust 1.96.1 and
Chrome 157.0.8103.38. This is a partial Material checkpoint.

| Check | Observed result |
| --- | --- |
| `./scripts/check.sh` | PASS: format, Clippy with warnings denied, 38 Rust cases, native compilation and both example WASM libraries |
| `./scripts/check-gpu.sh` | PASS on Metal: rounded stroke edge/interior, filled shapes, text, clipping and paint order |
| `cap cargo run --locked -p surface-material-gallery -- --smoke` | PASS: two native GPU frames |
| `./scripts/build-web.sh` and `./scripts/build-gallery.sh` | PASS: both browser packages built |
| `node scripts/browser-smoke.mjs` | PASS: original pointer/keyboard/theme/resize/scroll journey, no errors |
| `node scripts/browser-ime.mjs` | PASS: 40 composition/input events; current DOM export before and after Chinese commit; no errors |
| `node scripts/browser-material.mjs` | PASS: choices, disabled skip, manual groups, slider pointer/keys, button release, chips, both themes, dialogs/menus, mobile panel sizing and DOM export; no errors |

The 38-case run excludes one GPU test. That test was then explicitly run
and passed. Generated evidence is under `artifacts/`: `dom-export.json`,
`material-dom-export.json`, the browser JSON logs and gallery screenshots.
The export test also checks current tree links, deterministic reads,
controlled preedit/commit isolation and missing geometry after invalidation.
The dialog browser test repeats forward and reverse Tab cycles. It checks
both Rust focus and the actual canvas/textarea focus. Returning from an
input keeps keyboard focus on the canvas. Internal focus transfers do not
clear the runtime focus.

Linux hosted CI also passed at implementation commit
`7a57f4c65ad93569b735a86e6e5160f67792ea1d` in
[run 41773509204](https://github.com/faberline/surface/actions/runs/41773509204).
It ran format, Clippy with warnings denied, workspace all-feature tests
and WASM checks for both examples. The push run 41773481274 passed too.

The native gallery smoke proves window presentation, not every component
interaction. The own-window macOS Zhuyin prototype failed to produce a
preedit event. Native composition and its candidate popup remain unverified.
No Material family or full native editing acceptance is claimed.

## Chinese editing update

Recorded on 2026-11-04 on the same macOS environment below.

| Check | Observed result |
| --- | --- |
| `./scripts/check.sh` | PASS: format, Clippy with warnings denied, 28 Rust tests, native and WASM compilation |
| `./scripts/check-gpu.sh` | PASS: explicitly executed on Apple M1 Max / Metal |
| `cap cargo run --locked -p surface-showcase -- --smoke` | PASS: two presented native GPU frames |
| `node scripts/browser-smoke.mjs` | PASS: original pointer/keyboard/theme/resize/scroll journey; no errors |
| `node scripts/browser-ime.mjs` | PASS: Chrome 157.0.8103.38 composition, replacement, focus cancellation, grapheme deletion, undo/redo, and caret host placement; 40 events, no errors |

Native paste, caret movement, deletion and undo were observed in a visible
macOS window. Native OS composition and the candidate popup remain unverified.
See [text input](text-input.md) for the exact limits. This update does not
complete the full Material catalog or Core/Jet migration.

## Initial framework

Recorded on 2026-11-03, Asia/Taipei. This is evidence for the initial
standalone framework, not production acceptance of the Core/Jet backlog.

Environment: macOS on Apple M1 Max, Rust 1.96.1, wgpu 24.0.5,
glyphon 0.8.0, Taffy 0.7.7, wasm-bindgen CLI 0.2.129, Chrome 156.0.8078.80.

| Check | Observed result |
| --- | --- |
| `./scripts/check.sh` | PASS: format, Clippy with warnings denied, 19 Rust behavior tests, native compilation, WASM compilation |
| `./scripts/build-web.sh` | PASS: WASM binary and matching browser bindings generated |
| `./scripts/check-gpu.sh` | PASS: one explicitly executed GPU test on Apple M1 Max / Metal |
| `cap cargo run --locked -p surface-showcase -- --smoke` | PASS: real native window presented two GPU frames and exited successfully |
| `node scripts/browser-smoke.mjs` | PASS: real WASM/WebGPU journey, no page or console errors |

The ordinary Rust run deliberately excludes the GPU test. That test was
then run explicitly and passed. It verifies color pixels, rounded corners,
clipping, visible font rasterization, and rectangle/text paint order.
GPU access is required. A restricted-process run returned `NoAdapter`;
the run with OS GPU access passed. No missing adapter was treated as a skip.

The browser used a real Apple `metal-3` adapter, with
`isFallbackAdapter=false`. The browser smoke run uses a fresh headless
profile and the `--enable-unsafe-webgpu` test flag. It tested:

- Counter updates through pointer and keyboard input.
- Tab focus and disabled-control behavior.
- Controlled text input and backspace.
- Light/dark theme state.
- Device scale factor 2, with the canvas buffer matching its CSS size times
  the device scale. UI coordinates and pointer coordinates agree.
- Resize from 1000×900 to 480×600, followed by a successful control click.
- Wheel input that changed the actual semantic node position.

The final browser state was
`count=3;dark=true;name=Chri;nodes=18;focused=increment`.
Local generated evidence lives under `artifacts/`. The screenshot in
[`images/showcase.png`](images/showcase.png) is from that browser run.

## Re-run browser smoke

Build and serve the example as described in README. Install Playwright as
an optional local test tool:

```sh
npm install --no-save --package-lock=false playwright@1.61.1
npx playwright install chromium
node scripts/browser-smoke.mjs
```

`SURFACE_CHROME` can select an existing browser executable.
`SURFACE_PLAYWRIGHT` can select the ESM entry point of an external
Playwright installation. Tests start their own local HTTP server on a free
port. `SURFACE_URL` can select an already running test server.

## Remaining qualification

Windows/Linux native GPU runtime, other browsers, full IME editing,
screen readers, performance/load and release packaging remain open.
Core and Jet have draft migration PRs with local evidence. Hosted
consumer acceptance is separate. Linux builds and Rust behavior passed
at the input checkpoint below. This does not prove a Linux GPU/window journey.

The pinned dependency `block 0.1.6` reports a Rust future-compatibility
warning through the native graphics stack. Current Rust 1.96.1 checks pass.
Upgrade the graphics stack before claiming support for later toolchains.

## Hosted CI

Surface is public. [Run 41762321151](https://github.com/faberline/surface/actions/runs/41762321151)
passed at `1754a2e96e487ea693be558041abdf21ffc63d27`: Linux format,
Clippy, workspace all-feature tests and WASM check. This is the input
checkpoint, before the Material/export changes above. Later commits need
their own run.

### Historical startup block

[Initial run 41668157876](https://github.com/faberline/surface/actions/runs/41668157876)
ended with `failure` before executing any job step. GitHub's check
annotation states:

> The job was not started because recent account payments have failed or your spending limit needs to be increased.

That initial run remains a failed, unrun gate. The later successful run
above supplies separate evidence at its exact head.
