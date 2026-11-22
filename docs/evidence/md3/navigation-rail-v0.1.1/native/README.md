# Navigation Rail on macOS

The Surface 0.1.1 Gallery ran on macOS 27.2.1 with Apple M1 Max Metal. The release source manifest has SHA-256 `a7397c5a8bfb3c47874d0105eb772f0014b1e1925ac70e16674f65c96f7e9e96`. The native build input manifest used for replay has SHA-256 `34501ee07387faacb1e4b4643b13d9918602efcf2be78f19b58dfa24e7c8f59a`.

The Gallery, Material components, runtime and accessibility code use the release source. A disposable overlay adds a window-sizing example, a synthetic Winit adapter, and a `Page.screenshot` test adapter on the existing authenticated loopback transport. The test adapter uses Surface's extended display list from the presented frame and reads pixels back from Metal at one pixel per logical point. The published PNGs describe this fixture. The Surface 0.1.1 navigation release uses its existing CLI for semantic actions. The screenshot transport supplied by this fixture belongs to the test harness.

The harness sends settings and keyboard actions through the stock CLI. It uses synthetic Winit mouse events in the 704×512 windows and synthetic Winit touch events in the 320×240 modal. The AppKit collector reads the Gallery process's accessibility properties using existing permissions. Coverage includes the inspected AppKit tree. Listening to VoiceOver speech and physically operating an input device require a separate run.

## Replay

Run on a Mac with Metal, an active desktop, the Rust toolchain, Cap, and the Xcode command-line tools. The AppKit collector requires existing accessibility access to report the live tree.

```sh
shasum -a 256 -c evidence.sha256
sh build-and-run.sh /path/to/verified-surface /tmp/new-rail-captures
python3 -B audit-evidence.py /tmp/new-rail-captures/modal /tmp/new-rail-captures/inline
```

The replay uses the checkout only as an input. The build, session descriptor, temporary credential and binaries live in disposable directories under `/tmp`. The replay checks both release source manifests and the exact overlay hashes before it builds.

## Results

| Modal profile | Full → held panel (dp) | Held panel (PNG pixels) | Scrim alpha |
| --- | --- | --- | --- |
| Light, LTR, 704×512 | 286 → 188.76 | 188 | 82 → 54 |
| Dark, RTL, 704×512, 200% | 360 → 237.60 | 237 | 82 → 54 |
| Dark, RTL, 320×240, 200% | 320 → 211.20 | 211 | 82 → 54 |

The active modal kept Inbox focus and marked Home as the current page in AppKit while held. The background rejected activation. Cancel restored the starting geometry and ignored the stale release. A 28% slow release returned to the open position. A 64% slow release preserved modal focus during closing and restored focus to the opener after closing. The velocity threshold is outside this receipt's scope.

In the 320×240 case, the wheel moved the list scroll offset from 136 to 46 dp. The header stayed fixed. A vertical synthetic contact preserved the selection and left the modal open. The 200% Inbox label rendered two lines with 940 and 928 matching ink pixels.

Classic rail verified the 80 dp width, 64 dp items, keyboard selection and a visible vector selection pill. The inline wide rail expanded to 286 dp. The 320×240 compact case used the RTL bottom navigation at 200% text size and kept its targets at 48 dp or larger.

All 21 PNGs match their reported dimensions and SHA-256 checksums. The native build input manifest omits test files. The only later change in the broader source snapshot was an uncompiled accessibility test fixture.
