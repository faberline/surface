# Material, Chinese input, and consumer migration

The active goal is to complete all three workstreams below. A starter set,
an API-only test, or a compatible duplicate implementation does not close
this goal. Each row needs current source and matching execution evidence.

## Material 3

Use the [Material 3 component catalog](https://m3.material.io/components)
as the coverage baseline. Track every family and its variants: app bars,
badges, bottom sheets, buttons, button groups, cards, carousels, checkboxes,
chips, date pickers, dialogs, dividers, floating action buttons, icon
buttons, lists, loading/progress indicators, menus, navigation bars,
navigation drawers, navigation rails, radio buttons, search, segmented
buttons, side sheets, sliders, snackbars, switches, tabs, text fields,
time pickers, and tooltips. The rendered official catalog was inspected on
2026-11-04. It contains 36 families, including extended FABs, FAB menus,
split buttons and toolbars. Track that full inventory before declaring
Material complete.

Shared foundations include color roles and generation, typography, shapes,
elevation, motion, state layers, focus, selection, disabled/error behavior,
and adaptive sizing. Components must perform their named behavior through
the same Rust runtime on desktop and WebGPU. Static drawings alone do not
prove an interactive component. Provide a browsable component gallery and
tests for keyboard/pointer behavior, focus and overlay handling, state,
rendering, and both themes.

## Chinese text and IME

- Bundle licensed font coverage for traditional/simplified Chinese and
  common phonetic composition text on desktop and WASM.
- Keep committed app state separate from active preedit text.
- Handle composition begin, updates, selection, commit, cancel, and blur.
- Draw selection, caret and composition underline using the same shaping
  and geometry as text paint. Place the native candidate window at the caret.
- Support grapheme-safe cursor movement/deletion, selection replacement,
  pointer placement/drag, clipboard, and undo/redo.
- Use a browser input bridge for real composition and mobile keyboard
  integration. Prevent duplicate commits and ordinary-key interference.
- Run model tests, renderer tests, a real browser composition journey, and
  a native OS input-method journey. Record which OS/browser/input source ran.

## Core and Jet

- Pin both Jet packages to one standalone Surface revision. Keep generated
  Rust on that source too, so no generated app silently pulls the old Core types.
- Make Core's old package names forward to the same standalone types, or
  remove them after updating every consumer. Preserve API/type identity.
- Verify fresh remote state, discussions, PR overlap and AW ownership before
  changing each repository. Keep unrelated WIP intact.
- Run the relevant Core and Jet gates, including generated-Rust builds and
  applicable browser/DOM/WASM parity. A manifest edit alone is not migration.
- Publish reviewable branches/PRs with exact dependency pins and evidence.
  Do not mark the older Core issues accepted without their own full criteria.

## Evidence

Keep source pins, executed checks and remaining gaps in the verification
record. Surface is public. Linux hosted CI passes at the input checkpoint
`1754a2e96e487ea693be558041abdf21ffc63d27`. Later source revisions need their
own checks. Failed, skipped and unrun gates do not establish acceptance.
