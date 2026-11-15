# Roadmap

The current target is the full Material and Chinese input framework in
[the completion contract](completion-contract.md), with `UI = f(state)`
on desktop GPU and browser WebGPU.

| Stage | Scope | State |
| --- | --- | --- |
| Foundation | Semantic MicroDOM, IDs, typed styles, layout cache, clips, hit testing, ordered display list | Implemented; see verification record |
| Shared app | Immutable views, messages, state updates, focus, pointer, keyboard, controlled text, scroll | Implemented with the input limits in README |
| Material controls | Generated light/dark roles, button/chip variants, selections, slider, tabs, menus and dialogs | Working partial catalog; all family/variant qualification remains open |
| GPU and hosts | Native window, WASM canvas, shared text and rectangles, resize/DPI | Implemented; platform evidence is recorded separately |
| Editing and accessibility | Caret, selection, clipboard, IME, grapheme edits, platform accessibility bridges | Editing implemented; browser composition tested; native OS composition, mobile keyboards and accessibility bridges remain open |
| Component runtime | Keyed component identity, local state, effects with cleanup, async commands, batched scheduling | Planned; legacy hooks alone do not satisfy this stage |
| Material system | Theme tokens, color generation, typography, shapes, elevation, motion, ripples, complete component families | Color generation, borders, basic state layers, per-corner shapes with the Material shape tokens, the Material type scale (no heavier faces or rendered letter spacing yet) and partial motion (color and corner transitions, duration and easing tokens, reduced motion) implemented; remaining foundations and families open |
| Rich render and tooling | Images, transforms, damage, virtual lists, drag, Rust semantic driver, screenshots/traces, replay | Planned |
| DOM inspection | Current semantic tree, optional bounds/clips/paint order, interaction and editor state | JSON and typed exports; Rust and browser behavior checked |
| Adoption and release | Migrate Jet/Core consumers, qualify Windows/Linux/macOS and browsers, publish crates | Draft migration PRs published; full acceptance and release open |

Each later stage needs its own API contract and tests before a release can
claim support. Existing Core issue acceptance remains independent.

## Observability track

Decision: [0002-devtools-protocol](decisions/0002-devtools-protocol.md).
Design and items O1 to O18: [observability.md](observability.md).

The track makes a running app observable for AI agents and tests: trace,
screenshot, paint, input and wait, through a Surface-owned devtools
protocol. It starts after the desktop milestone S1 (#103 key handler,
done in PR #118; #48 native IME, open). Migration steps 13 and 9 come after
the first batch, O1 to O5. Web devtools waits for decision 0001 D-B to lift.

## Gaps to egui and iced

Order of the gaps between Surface and egui/iced. The order follows what the
infera desktop app, a real consumer, meets first. Only wave 1 is split into
issues now.

| Wave | Content | Existing issues | New in wave 1, or split later |
| --- | --- | --- | --- |
| 0 | Desktop slice S1 to S3 (in progress) | #103 (done, PR #118), #48, #105, #107, #108, #112, #116 (open) | none |
| O | Observability O1 to O18 | #68 (re-split); O1 to O8 are #125 to #132, open, with PRs #136 to #138 open | see observability.md |
| 1 | Desktop app basics | Migration step 11 (docs), #110, #111 | 1. Layout completion: margin, per-side padding and border, `flex_shrink`, space-between, `align_self`. 2. App-level shortcuts. 3. Right click and context menu. 4. Cursor icons. 5. Scrollbar painting. 6. Design of async command, timer and subscription. |
| 2 | Painting and text | #7, #8, #10, #11, #69, #106, #6 | Split later: opacity, image decoding, icons and vector paths, gradients |
| 3 | Large data and windows | #107, migration step 21 | Split later: virtual list, damage repaint, table, multiple windows, window options, native menus, component local state |
| 4 | Accessibility, platforms, release | #53, #54, #49, #50, #52, #112 | Split later: AccessKit, crates.io publishing, MSRV, CHANGELOG |

Migration steps 13 and 9 keep their place: after the first observability
batch (O1 to O5).
