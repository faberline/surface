# Initial framework verification

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
Playwright installation. `SURFACE_URL` defaults to `http://127.0.0.1:8080`.

## Remaining qualification

Windows/Linux native GPU runtime, other browsers, full IME editing,
screen readers, performance/load, release packaging, and Core/Jet consumer
migration are unrun or not implemented. The new CI workflow is configured to check Linux builds and behavior.
Its first hosted job did not start because of the account billing limit.
No Linux CI test ran. A passing future CI run would still not prove a
Linux GPU or window journey.

The pinned dependency `block 0.1.6` reports a Rust future-compatibility
warning through the native graphics stack. Current Rust 1.96.1 checks pass.
Upgrade the graphics stack before claiming support for later toolchains.

## Hosted CI block

[Initial run 41668157876](https://github.com/faberline/surface/actions/runs/41668157876)
ended with `failure` before executing any job step. GitHub's check
annotation states:

> The job was not started because recent account payments have failed or your spending limit needs to be increased.

The account owner must resolve the GitHub billing/spending restriction,
then re-run the workflow. This is an unrun cloud gate. The local evidence
above remains limited to the tested macOS and browser environments.
