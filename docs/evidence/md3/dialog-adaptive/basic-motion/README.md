# Basic Dialog motion

Basic, Alert and the nested discard prompt fade in and out. The panel and scrim paint separately. When a dialog leaves, focus and input go back to the previous scope while the outgoing panel still paints. Reopening resumes from the current frame. Reduce Motion reaches the endpoint immediately.

- The exact native workspace passed 1,277 tests. The separate Metal suite passed 13 tests. Two headless Gallery replays agreed on every PNG from Page.screenshot, CLI and MCP. All 24 window samples were zero. [Native tests, screenshots and replay](native/)
- Headless Chrome passed 542 focused checks across M256–M262. The tests verified 27 PNGs, entry and exit pixel colors, reversal, real text input during nested exit, and the Reduce Motion endpoints. [Browser proof and replay](web/)

The native code patch rebuilds from the [Runtime source](../runtime-preflight/) and verifies 572 source files. The test patch adds browser registration and verifies 574 files. Both exact patches independently reconstructed. The Full-screen FAB container transform has a separate verification lane.

Run `shasum -a 256 -c SHA256SUMS` from this folder to verify these published copies.
