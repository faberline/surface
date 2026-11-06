# Core and Jet migration

Surface has a new home: `faberline/surface`. This repository does not depend
on Core or Jet. Their working trees and release tags were not changed.

The existing `surface` and `ui-runtime` sources were imported from Core
commit `41c352eaae687a1914087d279f36adbe9e9b4af5`. Their public names remain
available. New APIs are additive. Imported shell-recursive EC wrappers were
replaced with direct Rust behavior tests suitable for this repository.

Core's and Jet's main branches still use the old dependency arrangement.
The reviewable migration proposals are:

- [Core PR #95](https://github.com/faberline/core/pull/95), implementation commit
  `23e614ca3e85e7ef77ae5563bb9c7a1fbdc15120`: compatibility packages
  re-export the standalone types and hooks, including debug support.
- [Jet PR #75](https://github.com/faberline/jet/pull/75), implementation commit
  `41ecdf95aeaa2723f862d1007fe2d1eb6c7718e1`: both UI dependencies and
  generated Rust use standalone Surface. `cli-std` stays on Core v0.4.14.

Both pin `1754a2e96e487ea693be558041abdf21ffc63d27`. Surface is public.
Its Linux CI passes at that revision. Neither consumer PR is merged.
That revision is not an ancestor of Surface main. Surface has no tags yet.
The plan below replaces the pin with a tag cut from main.

Core's mixed-source behavior/type tests pass locally. Core CI stops at
draft/issue preflight; old EC metadata and independent acceptance remain
open. Jet's complete local advanced WASM gate passes, including MUI/AntD
live parity. Its hosted parity job reports success by skipping an unwired
oracle, so it is not hosted parity acceptance. Three existing Clippy
findings also reproduce on Jet's untouched base.

Until the facades are adopted, old Core types and standalone types have
different Cargo identities. Move both UI packages together.

## Jet and Surface split

This plan was agreed on 2026-11-05. No step has started. Nothing in it is
accepted or complete. A measured spike, D1, decides whether Surface takes
over Jet's layout, text and GPU rendering.

Citations are pinned to exact commits. Plain paths are Surface main
`8b4e205`. Paths that start with `jet:` are Jet main `a584413`.

### Dependency direction

- Jet depends on Surface. Surface depends on no Jet code.
- Surface's only Jet-named dependency is the optional
  `jet-bridge-protocol`. It comes from the separate `faberline/jet-bridge`
  repository.
- Jet takes every Surface crate from one Surface tag on main.
- Core only re-exports during the move. Then Core retires its `surface` and
  `ui-runtime` crates.
- `cli-std` stays on Core. Jet pins it at Core v0.4.14
  (`jet:Cargo.toml:83`).

### Starting point

- Jet takes `surface` and `ui-runtime` from Core tag v0.4.14
  (`jet:wasm/Cargo.toml:74-75`). `jet-wasm` re-exports them
  (`jet:wasm/src/lib.rs:80-83`, `jet:wasm/src/react/mod.rs:17`).
  Generated Rust names only `jet_wasm` paths
  (`jet:src/tsx_to_rust/emit.rs:134-136`).
- Jet's WebGPU renderer is a v0. It measures text with hard-coded widths
  (`jet:wasm/src/renderer/mod.rs:1539-1546`). It paints procedural
  placeholder glyphs (`jet:crates/grid-wasm/src/renderer_bridge.rs:271-276`).
- Core commit `eaa4b1b` makes `Props` and `Component` fields private. It
  sits between Core v0.4.14 and `41c352e`. Jet's runtime and generated code
  still use the public fields.
- Surface's wasm32 check builds `surface` only as a dependency of its two
  examples (`scripts/check.sh:9`). No workspace crate depends on
  `ui-runtime`. So Surface never wasm-checks `ui-runtime`, or the `debug`
  feature that Jet enables (`jet:wasm/Cargo.toml:61`).
- Surface's GPU test is ignored by default
  (`crates/surface-wgpu/tests/it.rs:44`). Only `scripts/check-gpu.sh` runs
  it. No gate runs the browser smoke journey.
- In standalone Jet, `wasm_build_end_to_end` fails early. It looks for
  `jet-wasm` under `projects/jet/wasm`, which does not exist
  (`jet:src/wasm_build/mod.rs:468-474`). It fails before it reaches its
  `navigator.gpu` skip. No Jet CI job runs this test. The PR #75 head is
  not available locally, so its path handling was not rechecked.
- Jet's hosted parity job turns exit 77, an unwired oracle, into success
  (`jet:.github/workflows/parity.yml:23-25`).

### Phase 1: move the crates

Phase 1 changes where the crates come from. It does not change behavior.
Step numbers follow the agreed plan. A step lands only when its gate
passes. A skipped GPU or browser test never counts as a pass.

The frozen contract covers what Jet and Core use today.
[`consumer_contract.rs`](../crates/ui-runtime/tests/consumer_contract.rs)
pins each item and is the full list. Change it only with a consumer
migration. In summary:

- From `surface`: the `Element` variants, `Props` builders and getters,
  `Callback<P>`, `Component::new`, `name()` and `render()`, `ComponentFn`,
  and the `SurfaceSnapshot`, `SurfaceNode`, `SurfaceNodeKind`,
  `SurfaceProps` and `SurfaceRect` types.
- The `SurfaceSnapshot` JSON schema v1
  ([`snapshot_json.rs`](../crates/surface/tests/it/snapshot_json.rs)).
- From `ui_runtime`: `mount`, `MountHandle::flush` and `snapshot`,
  `set_update_scheduler`, the `use_*` hooks, `FiberId::new` and `get`, and
  the `debug` feature.

Steps:

1. **Jet: fix the standalone build path.** Resolve `jet-wasm` from the
   standalone layout (`jet:src/wasm_build/mod.rs:468-474`). Replace the
   `projects/cue` lookup (`jet:tests/wasm/wasm_build_end_to_end.rs:621-626`)
   with an in-repo fixture, or with an `#[ignore]` that gives a reason.
   Today that test skips when the fixture is absent.
   Gate: a clean standalone checkout builds a scaffolded app in
   `wasm_build_end_to_end`. The log shows a build, not a skip.
2. **Jet: add a required-WebGPU mode.** Add an opt-in flag, such as
   `JET_REQUIRE_WEBGPU=1`. With it set, a missing `navigator.gpu` fails the
   e2e and MUI/AntD tests. Today they return early
   (`jet:tests/wasm/wasm_build_end_to_end.rs:362-366` and `:772-776`).
   Jet's `scripts/verify-advanced-wasm-gates.sh` sets the flag. Non-GPU
   hosted runs leave it off.
   Gate: on a real GPU, the tests run and pass. In headless Chromium
   without WebGPU, the same command fails. Keep both logs. Needs step 1.
3. **Surface: freeze the consumer contract, then tag.** Add `ui-runtime`
   with all features to the wasm32 step in `scripts/check.sh` and in CI.
   Add a compile-only test that names every contract item. Check the
   [`provenance.json`](provenance.json) hashes, and list the known
   divergences. After merge, cut a tag on main.
   Gate: `check.sh` passes locally. Hosted CI passes at the tagged commit
   and runs the contract test and the provenance check.
   `git merge-base --is-ancestor <tag> origin/main` exits 0.
4. **Jet: absorb Core's private-field break.** Move both crates together
   from Core v0.4.14 to Core `41c352e`. Emit `Component::new` and
   `Props::default().with_*()`. Read `Props` fields through getters. Pass
   `FiberId::new` to `debug_snapshot_hooks`
   (`jet:wasm/src/debug/mod.rs:126`). Call `name()` instead of reading
   `c.name` (`jet:wasm/src/debug/mod.rs:232`). Keep Jet's re-exports
   unchanged. Reuse the PR #75 emitter diff if it matches.
   Gate: the `jet-wasm`, `tsx_to_rust` and `wasm_build` tests pass. The
   step 2 required-GPU gate passes, including MUI/AntD. `Cargo.lock` has
   one `surface` and one `ui-runtime`, both from Core `41c352e`.
   Needs step 2.
5. **Jet: point both crates at the Surface tag.** Change only
   `jet:wasm/Cargo.toml:74-75`, both lines in one change. Make no code
   change. `cli-std` stays on Core.
   Gate: `cargo tree -i surface` and `cargo tree -i ui-runtime` each show
   one package from Surface. The step 4 tests pass with the same counts and
   no new skips. Record the `.wasm` size change for the DOM and WebGPU
   builds. Needs steps 3 and 4.
6. **Core: re-point its facade at the Surface tag.** Update PR #95 from
   `1754a2e` to the tag. Core's crates stay `pub use surface_upstream::*;`.
   Gate: Core tests pass, including `ui-runtime` with `debug`. Core's
   `cargo tree` shows one `surface`. Needs step 3.
7. **Core: retire its two crates.** Delete the `surface` and `ui-runtime`
   facades, or mark them retired. Leave `cli-std` alone.
   Gate: Core CI passes. No local faberline repository has a Cargo
   dependency on Core's `surface` or `ui-runtime`. Needs steps 5 and 6.
8. **Surface: mark the migration complete.** Update this file,
   [delivery stages](delivery-stages.md) and [verification](verification.md).
   Replace the compatibility freeze with a semver promise for the Phase 1
   contract.
   Gate: the step 3 contract test still runs in hosted CI at the docs
   commit. Needs step 7.

### Phase 2: who owns what

Jet owns authoring:

- the TSX-to-Rust transpiler and the wasm build;
- CSS, through Jet's implementation of a `StyleResolver` trait;
- the DOM fallback renderer;
- the parity gates;
- `__jet_debug`.

Surface owns everything from `Element` lowering down to pixels:

- `Element` lowering, in a new `surface-element` crate;
- layout, text and GPU paint;
- focus, IME and accessibility.

This is the target. D1 decides whether Jet's renderer moves to it. If D1
fails, Jet keeps its own engine.

Surface never parses CSS. Its README excludes a general CSS engine
(`README.md:168`). Jet resolves styles and hands Surface a `surface::Style`.

The render seam is opt-in, behind a `jet-wasm` feature named
`surface-render`. All of it is new:

- `surface-element` depends only on `surface`, `ui-runtime` and
  `surface-app`.
- `trait StyleResolver { fn resolve(&mut self, tag: &str, props: &Props) -> Resolved }`.
  `Resolved` holds a `surface::Style` and the dropped CSS declarations.
- `fn lower(root: &Element, styles: &mut dyn StyleResolver) -> Result<View<ElementMsg>, LowerError>`.
- Roles, names and semantic ids come from `SurfaceSnapshot::from_element`
  (`crates/surface/src/snapshot.rs:24`). There is no second role table. The
  stable id is `Props::id`, else the snapshot `semantic_id`.
- `Element::Component`, duplicate ids and unknown roles return
  `LowerError`. They never panic.
- `enum ElementMsg { Click(id), Change(id, String), Checked(id, bool), Flush }`.
- `ElementApp` implements `surface_app::Application`. `update()` calls the
  callback, then `MountHandle::flush()`. `view()` only lowers the snapshot.
- Out-of-band writes through `ui_runtime::set_update_scheduler` post
  `ElementMsg::Flush` through a public, queued `surface-platform` proxy.
  The proxy is never re-entrant.
- Jet calls `surface_platform::run_runtime` with its canvas id and fonts.
  `jet-wasm` never imports `wgpu` or `winit` types.
- `surface_app::DomExport` JSON feeds `__jet_debug` and the layout-box
  comparator. Cross-renderer pixel diffs are not evidence.
- Jet's lock has one `surface`, one `ui-runtime` and one exact
  `wasm-bindgen`. `surface-wgpu` and `surface-platform` appear only under
  `surface-render`. Jet gets no bundled CJK font unless it asks for one.

### D1: who owns Jet's renderer

D1 is a measured spike. It decides whether Surface's layout, text and GPU
paint replace Jet's v0 renderer. Steps 9 to 18 are its prerequisites. Each
Surface change keeps existing goldens and JSON byte-identical.

Surface prerequisites:

9. **Optional bundled fonts.** `TextSystem::default()` embeds Tuffy and the
   16.4 MB Noto Sans CJK TC font (`crates/surface-wgpu/src/text.rs:11-12`).
   `Renderer::new` always uses it (`crates/surface-wgpu/src/lib.rs:133`).
   Put the fonts behind a default-on `bundled-fonts` feature. Let
   `Renderer` and `surface-platform` `Options` take caller font bytes.
   Gate: text tests pass on both paths. A test shows CJK as missing on the
   lean path. Record the wasm size of both. `scripts/check-gpu.sh` passes
   on a real adapter.
10. **Table and label roles.** Add `Table`, `Row`, `Cell`, `ColumnHeader`,
    `Label` and `Main` to `Role` (`crates/surface/src/microdom.rs:14-39`).
    `SurfaceSnapshot` already emits these strings
    (`crates/surface/src/snapshot.rs:151-157`).
    Gate: per-role tests cover focusability and `DomExport` JSON.
11. **Display, margin and per-edge box styles.** Add `display` (flex,
    block, none), margin, per-edge padding and per-edge border to `Style`.
    Today layout always uses Flex (`crates/surface/src/layout.rs:137`) and
    one padding value (`crates/surface/src/layout.rs:184`).
    Gate: layout tests assert exact rects for each new case.
    `scripts/check-gpu.sh` passes on a real adapter.
12. **A public out-of-band update path.** `Runtime::rebuild` is private
    (`crates/surface-app/src/lib.rs:218`). So is `UserEvent`
    (`crates/surface-platform/src/lib.rs:112`). Add a public `Runtime`
    method that applies a message outside input dispatch, then rebuilds.
    Add a public, queued `surface-platform` proxy that delivers it on the
    event loop.
    Gate: a headless test shows an outside write changing `DomExport`. A
    call made during `update()` is queued, not re-entered. A browser case
    shows a `setTimeout` write repainting, with `navigator.gpu` asserted.
13. **Configurable canvas and status ids.** The web host looks up
    `canvas#surface` (`crates/surface-platform/src/lib.rs:322`) and
    `#status` (`:196` and `:293`). Add both ids to `Options`
    (`crates/surface-platform/src/lib.rs:49-53`). Default them to `surface`
    and `status`.
    Gate: browser smoke runs on a page with a non-default canvas id and
    asserts `navigator.gpu` (`scripts/browser-smoke.mjs:19`).
14. **`surface-element` lowering.** Create the crate with `StyleResolver`,
    a `DefaultResolver` with no CSS, and `lower()`. Do not edit the
    provenance-tracked `snapshot.rs`.
    Gate: on a fixture corpus, the roles, names and ids in `DomExport`
    equal `SurfaceSnapshot`'s. Error fixtures return the expected
    `LowerError`. The wasm32 check covers the crate. Needs step 10.
15. **`ElementApp` over `MountHandle`.** Implement the `ElementMsg`
    contract. Connect `set_update_scheduler` to the step 12 proxy.
    Gate: headless `Runtime` tests use a fixed-metric `TextMeasurer` and no
    GPU. On counter, controlled-input and checkbox fixtures, pointer,
    focus-then-activate, text and timer-style writes each change
    `DomExport` as expected. Needs steps 12 and 14.

Jet prerequisites:

16. **Align `wasm-bindgen`.** Jet's scaffold pins `=0.2.121`
    (`jet:src/wasm_build/mod.rs:510-512`). `surface-platform` pins
    `=0.2.129` (`crates/surface-platform/Cargo.toml:30`). Cargo cannot
    unify two exact pins. Move the scaffold to `=0.2.129` with matching
    `wasm-bindgen-futures` and `js-sys`, or agree on one version.
    Gate: in required-GPU mode, `wasm_build_end_to_end` builds and loads a
    scaffolded app. The existing WebGPU and DOM paths still render.
    Needs step 2.
17. **Jet's `StyleResolver`.** Port Jet's inline-style and theme logic into
    `impl surface_element::StyleResolver`. Leave the old renderer as it is.
    Gate: native tests lower the MUI and AntD fixtures. Roles, names and
    ids match the DOM path exactly. A checked-in report lists every dropped
    declaration. Needs steps 5, 11 and 14.
18. **Opt-in `surface-render` path.** Add the non-default `jet-wasm`
    feature and a `Surface` variant of `WasmRenderer`
    (`jet:src/wasm_build/config.rs:39-49`). WebGPU stays the default.
    Gate: a required-GPU Chromium run on counter, input and checkbox shows
    input changing `DomExport` and a non-empty canvas readback. The run
    fails without `navigator.gpu`. Record the lean-font `.wasm` size.
    `cargo tree` shows one `wgpu` 24.0.5 and one `wasm-bindgen`.
    Needs steps 9, 13, 15 and 16.

The D1 gate:

19. **Jet: decide D1 with ADR-002 layout boxes.** Build layout-box JSON
    from `DomExport` and `ComputedDom` on `surface-render`. Compare it with
    `dom_app`. Each box must pass
    `abs(jet_v - dom_v) <= max(N_px, R * dom_dim)`
    (`jet:parity/data/docs/adr-002-per-renderer-baselines.md:129`). The run
    is a real browser with WebGPU required. The first verdict covers only
    what Surface can represent: counter, input, checkbox, and the MUI/AntD
    controls without tables. The hosted parity job must fail when its
    oracle is unwired. A Jet ADR records the verdict.
    Gate: a deliberately broken fixture fails the job. Per-renderer pHash
    baselines still pass on the v0 path. The ADR is merged.
    Needs steps 17 and 18.

If D1 passes:

20. **Surface: canvas accessibility companion tree
    ([#54](https://github.com/faberline/surface/issues/54)).** Mirror
    roles, names, states and focus into hidden DOM nodes beside the
    canvas. `MicroDom` stays free of web types.
    Gate: a Chromium test finds controls with `getByRole` and their names.
    Tab keeps DOM focus and Surface focus in step. The test does not skip.
21. **Surface: virtual list for List and Table.** Layout and paint touch
    only visible rows.
    Gate: on a 10k-row fixture with real WebGPU, the laid-out node count
    stays bounded. Frame time stays under a recorded budget while
    scrolling. Needs step 10.
22. **Surface: table selection and TSV copy.** This replaces Jet's
    fixture-only selection.
    Gate: `surface-app` tests cover selection and TSV output. A real-WebGPU
    browser run selects cells and reads the clipboard. Needs steps 10 and
    21.
23. **Jet: move `__jet_debug` onto `DomExport` and `DisplayList`.**
    `fiberTree` and `hookValues` keep reading `ui-runtime` debug.
    Gate: Jet's debug tests pass on `surface-render` in a required-GPU run.
    Surface's `scripts/bridge-e2e.py` passes against a Jet-built page.
    Needs step 18.
24. **Jet: make `surface-render` the default.** Keep the old renderer
    behind a non-default feature for one release.
    Gate: the advanced gates pass in required-GPU mode with zero GPU skips.
    MUI/AntD pHash and ADR-002 boxes pass on `surface-render`. The DOM
    fallback and the step 20 test pass on the default build. The lean-font
    bundle is within the agreed budget. Needs steps 19 to 23.
25. **Jet: delete its duplicate engine.** Do this one release after step
    24. Drop `taffy` 0.5 and `rustybuzz` (`jet:wasm/Cargo.toml:80` and
    `:85`). Leave the grid product alone.
    Gate: `cargo tree -p jet-wasm -e normal` shows one `taffy` (0.7.7), no
    `rustybuzz` and one `wgpu` 24.0.5.

If D1 fails, the step 19 ADR names the fallback. It replaces steps 20 to
25:

- Jet keeps its own engine.
- Jet moves its layout to `taffy` 0.7.7 and wires it into the live path.
- Jet replaces its placeholder glyph atlas with real glyphs.
- The losing stack is deleted.

### Open decisions

Each decision closes only with the named evidence.

- **Where lowering lives.** The plan uses Surface's `surface-element`. A
  Jet-owned adapter needs fewer cross-repo PRs. Evidence: during steps 14
  to 18, count the lowering changes that needed a Surface PR. Check for any
  role or name mismatch between `SurfaceSnapshot` and `DomExport`.
- **What counts as a D1 pass.** Pick `N_px`, `R` and the fixture set.
  ADR-002's defaults are `N_px = 2` and `R = 0.02`. Evidence: the
  run-to-run variance of `dom_app` against itself, and the step 17
  dropped-CSS report.
- **Whether to take the D1-fail branch.** Evidence: the step 19 results
  for layout boxes, interaction and size.
- **The shared `wasm-bindgen` version.** Moving Jet to `=0.2.129` is the
  simple choice. Evidence: step 16 builds and runs in required-GPU mode,
  and Jet's other scaffold pins resolve.
- **The bundle budget and Jet's font set.** Options are no CJK, a CJK
  subset, or lazy loading. Evidence: the sizes from steps 5, 9 and 18.
- **Whether to split `surface`** into an element-model crate and an engine
  crate. DOM-only Jet builds would then stop pulling `taffy` 0.7.7.
  Evidence: the DOM-only size change from step 5.
- **Where GPU and browser evidence comes from.** Options are a hosted GPU
  runner, or a written local process with saved logs. Evidence: runner
  availability and cost. Until then, only recorded required-mode runs
  count. Skips never count.
- **Jet ADR-004, ADR-005 and ADR-010.** They cover the focus proxy, the
  a11y emitter and the IME input proxy. They could be marked superseded by
  Surface's host and #54, or folded into #54. Evidence: compare each ADR
  with Surface's hidden-textarea IME host and the #54 plan.
- **Rebase or replace Jet PR #75/#76 and Core PR #95.** Evidence: fetch
  the PR #75 head `41ecdf95`, which is missing locally. Diff it against
  step 4.

### Rejected options

- A CSS subset parser in Surface. It conflicts with `README.md:168`.
- Pinning any consumer to `1754a2e`. It is not on main. A tag from main
  replaces it.
- A shared `pub(crate)` role table between `surface` and
  `surface-element`. `pub(crate)` does not cross crates. It would also
  edit the provenance-tracked `snapshot.rs`.
- Making the font change optional or last. It must land before any
  default flip.
- Calling `MountHandle::flush()` from `view()`. That re-runs hooks inside
  a pure view.
- A Jet-owned tag-to-role table. It could drift from `SurfaceSnapshot`.
- Jet driving `surface_wgpu::Renderer` from its own event loop. `wgpu`
  types would cross into Jet code. Jet would keep a duplicate host with
  its own focus and IME.
- Failing on a missing `navigator.gpu` unconditionally. That breaks every
  non-GPU hosted run.
- A pHash distance as the cross-renderer gate. ADR-002 forbids
  cross-renderer pixel diffs. pHash stays a per-renderer baseline.
- Keeping both engines with no end state.

## Issue reconciliation

Reconcile the existing Core issues #10–#13 and Jet #41 against the
accepted standalone implementation. Keep the issues open until their full
criteria have independent evidence.

The new runtime covers a useful part of those plans. It does not complete
their full transform/image/damage, drag/trace, and desktop E2E contracts.
