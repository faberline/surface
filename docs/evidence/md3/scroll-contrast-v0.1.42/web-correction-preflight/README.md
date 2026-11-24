# Corrected headless Chrome preflight

The corrected commit is `ab8615c8495d99a652f4fac38d4db37f08c82c7f`. The checked source manifest lists the 517 files that the focused test assembly used. The committed checkout matches that manifest.

- Showcase ran the stock browser input suite, I1–I20, with 57 passing checks. After composition, Chrome selection `[0,1]` mapped to Runtime bytes `[0,3]` for 中文.
- The focused Material runner passed M241–M247 with 794 checks and 28 independently verified Canvas frames. During a real forced-color redraw, the browser caret stayed at 4 while Runtime still held 3. After trusted keyup, both held 4.
- The selected composition underline used HighlightText across its central 92 device pixels. Normal mode restored the original pixels. The selected range kept its backward direction.
- The isolated trusted-key stress replay passed 50 rapid attempts and 50 attempts with per-key observation. The initial stock candidate accepted 94 of those 100 attempts.

[Input and contrast counts](focused-results.json) · [CRLF and Unicode checkpoints](browser-ime-line-endings.json) · [Caret redraw trace](controlled-arrow.json) · [Caret screenshot](forced-caret.png) · [Composition underline measurements](underline.json) · [Underline screenshot](forced-preedit-underline.png) · [Keyboard stress trace](selection-stress.json)

There were no reported browser page errors or leftover browser processes. The tests use headless Chrome on macOS and CDP composition. Native input methods, screen-reader speech, and the desktop display-settings notification still require separate runs.
