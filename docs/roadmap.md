# Roadmap

The current target is a usable early Rust application framework with
`UI = f(state)` on desktop GPU and browser WebGPU.

| Stage | Scope | State |
| --- | --- | --- |
| Foundation | Semantic MicroDOM, IDs, typed styles, layout cache, clips, hit testing, ordered display list | Implemented; see verification record |
| Shared app | Immutable views, messages, state updates, focus, pointer, keyboard, controlled text, scroll | Implemented with the input limits in README |
| Material starter | Light/dark palette, typography, buttons, cards, checkbox, text field | Implemented starter set |
| GPU and hosts | Native window, WASM canvas, shared text and rectangles, resize/DPI | Implemented; platform evidence is recorded separately |
| Editing and accessibility | Caret, selection, clipboard, full IME, grapheme edits, platform accessibility bridges | Planned |
| Component runtime | Keyed component identity, local state, effects with cleanup, async commands, batched scheduling | Planned; legacy hooks alone do not satisfy this stage |
| Material system | Theme tokens, color generation, elevation, motion, ripples, menus, dialogs, lists, navigation | Planned |
| Rich render and tooling | Images, transforms, damage, virtual lists, drag, Rust semantic driver, screenshots/traces, replay | Planned |
| Adoption and release | Migrate Jet/Core consumers, qualify Windows/Linux/macOS and browsers, publish crates | Planned |

Each later stage needs its own API contract and tests before a release can
claim support. Existing Core issue acceptance remains independent.
