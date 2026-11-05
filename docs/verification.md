# Framework verification

## Motion, shapes and typography combined

Recorded on 2026-11-05, on macOS / Apple M1 Max (Metal) and Rust 1.96.1.
The three sections below this one (Motion, Shapes, Typography) were each
recorded with only that feature present. Their test counts and wording
describe those separate runs. This section records the combined tree.

| Check | Observed result |
| --- | --- |
| `./scripts/check.sh` | PASS: format, Clippy with warnings denied, 107 Rust tests passed and 4 GPU tests ignored, native compilation and both example WASM libraries |
| `./scripts/check-gpu.sh` | PASS on Apple M1 Max / Metal: `gpu_offscreen_paints_clips_rounding_text_and_order`, `gpu_offscreen_per_corner_shapes`, `gpu_offscreen_bottom_corners_are_not_swapped` and `gpu_offscreen_motion_animates_color_and_corners` all ran with 0 ignored, and each printed a `GPU evidence: AdapterInfo` line |

No existing test assertion changed when the features were combined.
`Style` now holds `corners`, `transition` and four text fields. All six are
omitted from JSON at their defaults. The per-feature JSON tests run against
the combined `Style`, so a style that uses none of them exports as before.

Two new tests in `crates/surface-app/tests/features.rs` run all three
features in one `Runtime`. A node has `corners`, a `radius` that changes, a
styled label and a color `transition`. They check that:

- the blended display copy keeps the corner shape and paints a `Shape` op
  whose color is part-way between the two targets;
- the label paints as `StyledText` with its size, line height and weight,
  and its color blends too;
- layout keeps the target style and the 32 pixel line height while colors
  blend, so the bounds do not move.

The Material components now carry a shape token, a state change transition
and a type role together. Their `corners` are blended too. See
[Material motion](material.md#motion).

What this does not show:

- The GPU tests draw per-corner shapes, the older rounded, clipped and
  text scene, and a part-way color and corner transition (see the Motion
  section). No GPU run drew `StyledText`.
- No browser run was made for the combined tree. The WASM check proves
  compilation only.
- The headless clock tests do not measure frame pacing.
- No screenshot of the combined Material components was taken.

## Motion

Recorded on 2026-11-04, on macOS / Apple M1 Max and Rust 1.96.1. This is
a partial motion checkpoint. It does not complete the Material motion
foundation. See [Material components](material.md#motion) for the gaps.

| Check | Observed result |
| --- | --- |
| `./scripts/check.sh` | PASS: format, Clippy with warnings denied, 62 Rust tests (1 GPU test ignored), native compilation and both example WASM libraries |
| `./scripts/check-gpu.sh` | PASS on Apple M1 Max / Metal. This is the existing offscreen paint test. It does not draw a transition. |

17 of the 62 tests cover motion: 5 for easing, validity, JSON and layout in
`surface`, 10 for the runtime in `surface-app`, and 2 for the Material
tokens and component adoption.

The runtime tests use a headless clock. They call `Runtime::set_time` with
chosen millisecond values and read the display list from `render()`. They
prove the blending, retargeting, reduced-motion and export rules. They do
not prove frame pacing. No test measures how often a native window or a
browser asks for and presents frames.

Corner motion was added later on 2026-11-05. Corners now blend, and `FULL`
resolves against the node size on each frame. Eleven runtime tests (C1 to C11)
in `crates/surface-app/tests/motion.rs` check this with the headless clock.
One GPU test, `gpu_offscreen_motion_animates_color_and_corners`, drives a
`Runtime` with the real offscreen renderer. It ran on Apple M1 Max / Metal
with 0 ignored and printed a `GPU evidence: AdapterInfo` line. At t=100 ms
the center pixel was within 3 per channel of (128, 0, 128). The corner
probes matched radius 20, between the start (0) and end (40) radii. With
`SURFACE_GPU_EVIDENCE` set, it also writes `gpu-motion-midpoint.png`.

Not verified:

- The native redraw loop (`needs_frame` then `request_redraw`) and the
  native clock were compiled, not run in a window.
- The browser clock, the `requestAnimationFrame` loop and the
  `prefers-reduced-motion` change listener passed the WASM compile check.
  They were not run in a browser.
- `SURFACE_REDUCED_MOTION=1` on native was not exercised by a test.
- No browser or native window screenshot shows a part-way transition.
- Linux and Windows hosts are unchecked.

## Shapes: per-corner radii and Material shape tokens

Recorded on 2026-11-04, on macOS / Apple M1 Max (Metal), Rust 1.96.1.
This covers `surface::Corners`, the `PaintOp::Shape` and
`PaintOp::ShapeStroke` commands, the per-corner shader and the
`surface_material::shape` tokens.

| Check | Observed result |
| --- | --- |
| `./scripts/check.sh` | PASS: format, Clippy with warnings denied, workspace tests, WASM compilation of both example libraries |
| `./scripts/check-gpu.sh` | PASS on Apple M1 Max / Metal at the time of this section: the two GPU tests that existed then ran and passed. The script now runs 4 tests; see the first table in this file |

New Rust cases: 11 in `crates/surface/tests/it/shape.rs` (constructors,
resolve clamping, validation, JSON, legacy fallback, paint ops, state layer,
focus ring on legacy and corner nodes, parent clip) and 3 in `crates/surface-material/tests/shape.rs`
(token values, component mapping, a rendered top-rounded text field).

`gpu_offscreen_per_corner_shapes` renders a filled shape with two diagonal
rounded corners and a stroked shape with the other two. It reads back pixels.
It checks that rounded corners are empty, square corners are filled, and
that the interior and stroke edges are the expected color. The older
`gpu_offscreen_paints_clips_rounding_text_and_order` test is unchanged and
still passes, so uniform radii keep their pixels. `check-gpu.sh` now runs
every ignored test whose name contains `gpu_offscreen`. With
`SURFACE_GPU_EVIDENCE` set, the new test also writes `gpu-shapes.png`
next to `gpu-offscreen.png` under `artifacts/`.

These results do not show:

- Browser WebGPU behavior. The shader change was run on native Metal only.
  `scripts/browser-material.mjs` was not re-run for this change.
- Any adapter other than the Apple M1 Max.
- Rounded clipping. Children and text are still clipped by the rectangular
  parent clip.
- RTL mirroring of the `start` and `end` tokens.

## Typography and Material type scale

Recorded on 2026-11-04, on macOS / Apple M1 Max and Rust 1.96.1. This
covers text styles, line height, weights and the Material type scale.

| Check | Observed result |
| --- | --- |
| `./scripts/check.sh` | PASS: format, Clippy with warnings denied, workspace all-feature tests (56 run, 1 GPU test excluded) and the WASM check for both examples |
| `./scripts/check-gpu.sh` | PASS on Apple M1 Max / Metal: the existing offscreen test, explicitly run |

The new Rust tests cover:

- `TextStyle` defaults, and `Style` JSON that omits the new fields at their
  defaults and round-trips.
- Layout rejection of an invalid weight, line height or letter spacing.
- A size-only `TextMeasurer` still compiling and measuring as before, with
  the line height rescaling its height.
- Layout measuring with each node's own text style.
- Paint keeping the old `Text` ops for size-only styles and emitting
  `StyledText` for any other style.
- Editing caret height following the styled line height.
- `TextSystem` applying line height, weight and family. The weight test loads
  a patched copy of the bundled font, built in memory, whose glyphs are
  twice as wide, and checks the measured width. The family test measures the
  same text in Tuffy and in Noto Sans CJK TC and requires different widths.
  It also checks that a named family picks the nearest weight inside its own
  family, that an unknown name measures like the default family, and that
  Han text in a Tuffy request still falls back to a face that has the glyphs.
- All 30 type scale constants against the androidx values, and the theme's
  component adoption of those roles.

Existing tests needed no assertion changes for the Material adoption.

What this does not show:

- The GPU test is the existing one. It does not paint `StyledText` or a
  weighted face, so styled text on a GPU is unverified.
- No browser run was made. The WASM check proves compilation only.
- Letter spacing is not rendered, and Medium and Bold render with the
  regular face. See [Material components](material.md#typography).
- Measured widths show that a named family reaches a different face. No
  screenshot was taken of Tuffy text or of a mix of the two typefaces.
- The default family treats the bundled Tuffy as a regular face even though
  it declares weight 500. The test does not judge whether that is the right
  look. It pins that every default-family weight measures the same until a
  heavier face is added.
- Native and browser screenshots of the new type scale were not taken.

The behavior of weight and family matching was read from the cosmic-text
0.12.1 source and checked with a throwaway shaping run before the fix. In
that run `Family::Name("Tuffy")` at weight 400 shaped with Noto Sans CJK TC.
At weights 500 and 700 the same request shaped with Tuffy. That is why the
text system resolves the exact face weight itself.

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
