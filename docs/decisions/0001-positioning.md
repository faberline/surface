# 0001: Surface product positioning

Status: Proposed (awaiting owner decision)
Date: 2026-11-11

This record proposes a layering. It does not decide. The owner answers the
open decisions in the last sections.

## Context

Surface does four jobs at once. Line numbers below are from `origin/main`
at `bdf9693`.

### 1. Renderer and runtime for Jet and Core

- `crates/ui-runtime/tests/consumer_contract.rs` freezes the API that Core
  imports (see the header note at `:10`).
- `docs/migration.md` splits Jet and Surface (`:37`). Phase 2 and the D1
  spike (`:249`) decide who owns Jet's renderer. Steps 9 to 15 are the
  Surface prerequisites (`:257`-`:298`).

### 2. Web canvas host

- `surface-platform` runs on wasm32 with WebGPU.
- Safari IME work: #90, #91, #94, #95. CJK font subset: #89
  (`docs/web-font.md:46`). Browser e2e runs from Rust (#64).
- The canvas id is hard-coded: `get_element_by_id("surface")` at
  `crates/surface-platform/src/lib.rs:363`, and `"status"` at `:225`
  and `:334`. Migration step 13 (`docs/migration.md:285`) makes both
  configurable.

### 3. Material component library

- `crates/surface-material`.
- Foundations #6 (typography), #7 (shape), #8 (elevation), #10 (ripple)
  and #11 (state coverage) are blocked. About 25 component issues depend
  on them.

### 4. Desktop app framework

- `crates/surface-app` and `crates/surface-platform` (winit window).
- A first external consumer, the infera desktop app (macOS, now on egui),
  asked for this path.
- Gaps, checked against the code:
  - #102 wake and out-of-band update. `UserEvent` is a private enum
    (`crates/surface-platform/src/lib.rs:118`). `Runtime::rebuild` is
    private (`crates/surface-app/src/lib.rs:221`). Migration step 12
    describes the same gap (`docs/migration.md:276`); its line numbers
    (`:218`, `:112`) are stale.
  - #103 key handler. The `Application` trait has only `title`,
    `update` and `view` (`crates/surface-app/src/lib.rs:77`-`:84`). An
    app cannot see key events.
  - #104 close hook. `WindowEvent::CloseRequested => event_loop.exit()`
    (`crates/surface-platform/src/lib.rs:448`). The app cannot clean up
    or veto.
  - #48 native IME is unverified. `docs/text-input.md:49` says browser
    tests are not proof of a native candidate-popup journey.
    `docs/verification.md:209` records a failed own-window Zhuyin
    prototype.
  - #53 accessibility. `docs/roadmap.md:13` lists platform accessibility
    bridges as open.
  - #49 and #50 Windows and Linux are unqualified.
    `docs/verification.md:85`: "Linux and Windows hosts are unchecked."

## Problem

The four roles are not layered and not ranked. Work has leaned to the web
host and to Jet, because D1 needs them. The native desktop app path stays
thin. A real consumer now waits on it.

## Proposed layering

This is a proposal.

| Layer | Crate | Owns |
| --- | --- | --- |
| Core | `surface`, `ui-runtime` | MicroDOM, layout, text, display list. No platform APIs. |
| Renderer | `surface-wgpu` | Device, shaders, glyph atlas. |
| Hosts | `surface-platform` | Web canvas host and desktop (winit) host. Both first-class. |
| App framework | `surface-app` | Application, update, events, lifecycle. |
| Components | `surface-material` | Optional component library. |

Existing rules stay:

- wgpu types stay in `surface-wgpu`.
- Platform types stay in `surface-platform`.
- Surface never parses CSS.
- The API imported by Core stays compatible until the migration is done.

## Option noted, not recommended

"Desktop only" is not recommended by surface-service-desk. D1 compares
renderers in a browser, and the core is already host-independent. This is
the desk's recommendation. It is not a decision.

## Open decisions for the owner

These are unanswered.

- **D-A.** Is desktop a first-class host?
  - Yes: #102, #103, #104 and #48 go before the Material foundations.
  - No: desktop stays best-effort, and the infera consumer waits.
- **D-B.** Is the web host scope limited to what D1 needs until D1
  reports?
  - Yes: no new web features beyond migration steps 9-15 and bug fixes.
  - No: web grows in parallel.
- **D-C.** Which desktop platforms come first?
  - macOS only.
  - macOS, then Windows and Linux (#49, #50).

The Jet part of this is cross-repo. It goes through the planner and
aw-service-desk. This record does not change Jet.

## Consequences per answer

Wave 2 is migration steps 9-15. Wave 3 is the Material foundation splits.

- **D-A yes:** a desktop slice (#102, #103, #104, #48) goes first or beside
  Wave 2. #102 overlaps step 12, so one change serves both. Wave 3 slips.
- **D-A no:** Wave 2 then Wave 3 as planned. The infera consumer has no
  supported path.
- **D-B yes:** Wave 2 stays small. Web items outside D1 wait.
- **D-B no:** web items interleave with Wave 2. Wave 2 may take longer.
- **D-C macOS only:** #49 and #50 stay unscheduled. Verification stays on
  one platform.
- **D-C also Windows/Linux:** #49 and #50 join the desktop slice. Each
  needs real-device verification (#99).
