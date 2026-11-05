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
