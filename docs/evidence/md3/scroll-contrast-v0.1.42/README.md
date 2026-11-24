# Surface v0.1.42 Scroll Field display contrast

Candidate commit `f5b2e24d298c60a06d7b85635baf7a47657e31e2`. This is the pre-release verification record for PR #217.

Two exact native offscreen replays and 11 Metal tests passed. The stock and diagnostic captures use the same candidate source. The stock Chrome suite passed four cases and failed Material at M246. The full local workspace check was stopped after the browser confirmed the issue. This source is not ready to release.

[Native screenshots and replay commands](native/README.md) · [Candidate PR](https://github.com/faberline/surface/pull/217)

## Browser defect

A trusted ArrowRight advanced the focused textarea caret from offset 3 to 4. A real redraw occurred before Chrome sent its selection event. Surface then wrote offset 3 back to the browser. Keyup retained the wrong offset. The draft and focus stayed intact. The trace records an unsuppressed event run and a controlled timing comparison.

[Exact trace](web-initial/selection-event-trace.json) · [Caret screenshot](web-initial/m246-caret-offset-3.png)
