# Material Dialog native headless replay

This proof uses the pinned Gallery from the 563-file Dialog scratch build. Its source manifest SHA-256 is `46b4b82931a1f84977693a38d6e42f5c591795aa9ee27ad63e280d578a6cd802`. The Gallery and CLI binary manifest SHA-256 is `6953c3a5e078b067655fddb9d8dbd42bb386d41f3f5214bcfe384caef4d4da1c`. The replays rechecked all source and binary digests before and after capture.

Two independent replays produced identical reports and eight identical PNGs. Page.screenshot, CLI, and MCP returned identical bytes and metadata for each image. All 24 Mac visible-window checks returned zero. The verifier uses the stock headless Metal renderer and native semantic keyboard and text actions.

| Profile | Basic | Adaptive edit |
| --- | --- | --- |
| 960×800 light LTR | 400×536 panel, 368 dp scroll region, 48 dp side-by-side actions | Centered 400×420 Basic panel, visible 56 dp editor and 48 dp actions |
| 320×400 dark RTL at 200% | 280×352 panel, 96 dp scroll region, fully clipped tail, two 48 dp stacked actions | Square 320×400 Full-screen panel, 120 dp header, 72 dp editor, 48 dp actions |

The screenshots show actual title and action ink. Sampled Basic light fill is RGBA `[236, 230, 238, 255]`. Basic dark fill is `[43, 41, 47, 255]`. Compact Full-screen dark fills both canvas corners with `[20, 18, 24, 255]`. The compact Close icon is 24 dp from logical start.

The alert stays open on Escape, blocks a background semantic action and restores focus to its opener after acknowledgement. Dirty edit Escape opens a nested Basic confirmation and blocks the underlying Save action. Continue returns focus to the editor and preserves the new text. Save restores its opener and persists the draft across reopening. Discarding a different draft leaves the saved name in place.

The compact images expose a visual limitation. The RTL action order is correct, but Basic titles still paint left within their text bounds. The Full-screen two-line CJK title also paints left within its slot. The compact discard supporting text uses a scroll area, so its second line sits below the initial clip. These points are visible in the captures. The native replay checks clip and semantic delivery. Real wheel, browser, OS IME, and VoiceOver are separate checks.

## Reproduce

See the [source reconstruction and replay commands](../README.md#reproduce).

- [Native report](exact/replay-1/native-dialog-report.json) SHA-256 `eb6d35ae246b1a8f6aa8fe29535aef6f876a5bf64919ab3b8519a27c415da103`
- [Repeat digests](exact/repeat.sha256) SHA-256 `7edc3553a7294a770808408179eb80787edfda302f4e276b77694c43300ef55f`
- [Compact Basic](exact/replay-1/compact-dark-rtl-200-basic.png), [compact edit](exact/replay-1/compact-dark-rtl-200-edit.png), [compact alert](exact/replay-1/compact-dark-rtl-200-alert.png), and [compact discard](exact/replay-1/compact-dark-rtl-200-discard.png)
