# Surface v0.1.42 Scroll Field display contrast

## Corrected candidate

Draft PR #217 now points to `ab8615c8495d99a652f4fac38d4db37f08c82c7f`. The candidate keeps the browser caret at the new position during a forced-color redraw. It accepts selection after composition and preserves positions in authored CRLF and CR text.

The focused headless Chrome runs passed 20 Showcase input steps and seven Material contrast steps. The isolated keyboard stress replay passed 100 of 100 attempts.

Eleven Metal checks and two independent native offscreen replays passed on this commit. The three screenshot transports agree on every capture. The eight PNGs match between replays. All 28 window checks reported zero.

All five stock Mac headless cases passed 16,103 checks. Material passed 246 steps and 15,842 checks. Both Linux CI runs passed eight cases and 16,480 checks each. Both runs used the same candidate tree. The full local workspace check is running.

[Correction proof](web-correction-preflight/README.md) · [Corrected native replay](native-corrected/README.md) · [Full Mac headless tests](web/README.md) · [Linux CI replays](hosted/README.md) · [Candidate PR](https://github.com/faberline/surface/pull/217)

## Initial candidate

Initial commit `f5b2e24d298c60a06d7b85635baf7a47657e31e2`. This section preserves the source and result of the first verification.

Two exact native offscreen replays and 11 Metal tests passed. The stock and diagnostic captures use the same candidate source. The stock Chrome suite passed four cases and failed Material at M246. The full local workspace check was stopped after the browser confirmed the issue. This source is not ready to release.

[Native screenshots and replay commands](native/README.md) · [Candidate PR](https://github.com/faberline/surface/pull/217)

## Browser defect

A trusted ArrowRight advanced the focused textarea caret from offset 3 to 4. A real redraw occurred before Chrome sent its selection event. Surface then wrote offset 3 back to the browser. Keyup retained the wrong offset. The draft and focus stayed intact. The trace records an unsuppressed event run and a controlled timing comparison.

[Exact trace](web-initial/selection-event-trace.json) · [Caret screenshot](web-initial/m246-caret-offset-3.png)
