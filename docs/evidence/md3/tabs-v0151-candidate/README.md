# Material 3 Tabs v0.1.51 candidate

Primary and Secondary bars support fixed and scrollable layouts. Each bar
keeps one selected ID in the app. The active panel mounts beside the bar.
Tabs have stable Web panel links and native AccessKit relations. The
native scroll area retains its page actions and gives the tab list its
visible and revealable tab members.

This package starts with the 616-file candidate and its patch against
the sealed Radio v0.1.50 source. Decompress `surface-v0.1.51.patch.gz`.
Apply the patch from the Radio source and verify the result against
`source.sha256`. The two Git trees are recorded
in `radio-base-tree` and `candidate-tree`.

## Local verification

The workspace gate passed 1,368 Rust tests, formatting, Clippy, source
provenance, and native and Wasm checks. The real Apple Metal lane passed
16 offscreen GPU tests. [Workspace log](workspace-exact.log) ·
[Metal log](gpu-v0151.log.gz)

Two headless runs each tested a light 960 dp fixed bar and a dark 320 dp
RTL scroller with 200 percent text. The replay compared PNG bytes from
Page.screenshot, the CLI, and MCP. Both runs matched. Each run recorded
six zero-window samples. [Native replay 1](native/replay-1/native-gallery-report.json)
· [Native replay 2](native/replay-2/native-gallery-report.json)

[Fixed light](native/replay-1/wide-light-fixed.png) ·
[Dark RTL](native/replay-1/compact-dark-rtl-200.png)

The scoped Chrome WebGPU journey passed 371 checks and four Material
steps on Apple Metal. It covers linked tabs and panels, keyboard
selection, motion, Reduce Motion, real touch and wheel, and RTL focus
reveal. This run uses the development Gallery package and a runner scoped
to the four Tabs steps. [Scoped log](chrome-scoped/runner.log) ·
[Scoped receipt](chrome-scoped/material-tabs.json)

[Wide light](chrome-scoped/material-tabs-fixed-light.png) ·
[Wide dark](chrome-scoped/material-tabs-fixed-dark.png) ·
[Pressed](chrome-scoped/material-tabs-dark-pressed.png) ·
[320 dp RTL](chrome-scoped/material-tabs-rtl-320.png)

## Full macOS browser run

The exact v0.1.51 packages passed five headless suites and 20,664
checks. The Material suite ran 288 steps. Chrome WebGPU used Apple
Metal. The observer tracked primary Chrome, detached Bridge Chrome,
and the native Gallery. It recorded zero visible windows and zero
surviving processes.

[Full runner log](stock-macos/runner.log.gz) ·
[Window observation](stock-macos/headless-window-observation.json) ·
[Independent verification](stock-macos/verified-tabs-offline.json)

[Light fixed](stock-macos/artifacts/material-tabs-fixed-light.png) ·
[Dark fixed](stock-macos/artifacts/material-tabs-fixed-dark.png) ·
[Pressed](stock-macos/artifacts/material-tabs-dark-pressed.png) ·
[320 dp RTL](stock-macos/artifacts/material-tabs-rtl-320.png)

The portable original archive has manifest digest
`90fdec5ed8a20f716c6aac1dea168ebb30a093b159bfc3cc2f26ed4baf8a5e19`. This GitHub record preserves its source and binary pins,
original runner log, window and launch receipts, and representative
screenshots. The full inventory is in
[the archive manifest](stock-macos/full-local-archive.sha256).

## Rerun

Use a fresh directory under `/tmp` for the Rust build output and PNGs.
Use the exact candidate source. Run the five named headless browser and
bridge cases for a local test without windows.

```sh
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
  --features surface-material-gallery/devtools --bins
cap python3 -B "$proof/native/scripts/replay-tabs.py" \
  --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
  --cli "$CARGO_TARGET_DIR/debug/surface-devtools" \
  --out "$replay"
cap cargo xtask web e2e smoke ime material bridge-web devtools-headless \
  --artifacts "$browser_artifacts"
```

The coverage ledger remains partial. Optional badges, panel swipe,
system high contrast, and live screen-reader speech remain open.
[Material 3 Tabs](https://m3.material.io/components/tabs/specs) ·
[WAI Tabs pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/)
