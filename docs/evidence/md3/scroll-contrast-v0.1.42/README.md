# Surface v0.1.42 Scroll Field display contrast

## Current candidate after the cancellation repair

[PR #217](https://github.com/faberline/surface/pull/217) points to `86bf5eb86a5d070245db039b37977198ba1a417e`, tree `93b47517d26662b1e9aa5502f730349723fb0978`. The [549-file source manifest](candidate-cancel-fixed-source.sha256) has SHA-256 `c99ed8ff89b0cee66a84732598ac2ba652802c2e52c9122c3c5bb27f250ccd4b`. The fix preserves the authored text and selection immediately when a browser composition is canceled.

- Two stock offscreen Mac replays produced eight identical PNGs and identical reports. Page.screenshot, CLI and MCP agreed. All 28 visible-window counts were zero. Eleven separately run Metal tests passed. [Native Gallery](native-cancel-fixed/README.md) · [Metal receipts](gpu-cancel-fixed/README.md)
- Stock Mac Chrome passed five suites and 16,123 checks. Both I11 regressions and the Material contrast pixel checks passed. [Mac stock receipt](web-cancel-fixed/README.md)
- Both exact Linux workflows passed build-and-behavior and eight stock suites. Each ran 16,500 checks, including 77 IME checks. Both checkout trees match the exact candidate. [Hosted receipts](hosted-cancel-fixed/README.md)

The local full-workspace gate was still running when these results were archived. Live OS preference-change events, native input methods and screen-reader speech need separate evidence.

## Earlier cancellation regression

This issue held the earlier draft. A trusted Select All after a canceled composition can lose its selection during a real redraw. The controlled replay passed on v0.1.41 and failed on this draft. [Reproduction and traces](web-cancel-regression/README.md) · [Repair preflight](web-cancel-fix-preflight/README.md)

## Superseded caret and line-ending candidate

The earlier draft pointed to `ab8615c8495d99a652f4fac38d4db37f08c82c7f`. The candidate keeps the browser caret at the new position during a forced-color redraw. It accepts selection after composition and preserves positions in authored CRLF and CR text.

The focused headless Chrome runs passed 20 Showcase input steps and seven Material contrast steps. The isolated keyboard stress replay passed 100 of 100 attempts.

Eleven Metal checks and two independent native offscreen replays passed on this commit. The three screenshot transports agree on every capture. The eight PNGs match between replays. All 28 window checks reported zero.

All five stock Mac headless cases passed 16,103 checks. Material passed 246 steps and 15,842 checks. Both Linux CI runs passed eight cases and 16,480 checks each. Both runs used the same candidate tree. These runs use the superseded source.

[Correction proof](web-correction-preflight/README.md) · [Corrected native replay](native-corrected/README.md) · [Full Mac headless tests](web/README.md) · [Linux CI replays](hosted/README.md) · [Candidate PR](https://github.com/faberline/surface/pull/217)

## Initial candidate

Initial commit `f5b2e24d298c60a06d7b85635baf7a47657e31e2`. This section preserves the source and result of the first verification.

Two exact native offscreen replays and 11 Metal tests passed. The stock and diagnostic captures use the same candidate source. The stock Chrome suite passed four cases and failed Material at M246. The full local workspace check was stopped after the browser confirmed the issue. This source is not ready to release.

[Native screenshots and replay commands](native/README.md) · [Candidate PR](https://github.com/faberline/surface/pull/217)

## Browser defect

A trusted ArrowRight advanced the focused textarea caret from offset 3 to 4. A real redraw occurred before Chrome sent its selection event. Surface then wrote offset 3 back to the browser. Keyup retained the wrong offset. The draft and focus stayed intact. The trace records an unsuppressed event run and a controlled timing comparison.

[Exact trace](web-initial/selection-event-trace.json) · [Caret screenshot](web-initial/m246-caret-offset-3.png)
