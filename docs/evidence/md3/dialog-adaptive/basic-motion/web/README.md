# Basic Dialog motion in headless Chrome

This proof uses the [572-file Basic Motion source](motion-base.sha256), SHA-256 `6e9faca3b07a7057f652c18967a9109a23c7ad4f0124859789dc4696e1f7f044`. The [complete M256–M262 browser patch](dialog-motion-complete-e2e.patch) has SHA-256 `8fa427f82fe84bafff8b4aa3945b83c86074df4ada2cf07b2ab3493259712715`. The [incremental M262 patch](m262-dialog-motion-e2e.patch) applies atop the previously sealed M256–M261 source. The [574-file source result](source.sha256) SHA-256 is `6f4537581935acb9f47deab909192a0bed69c53cff6c3fdd6cb75b2e73d5e2ed`. Only three `e2e` files differ from the Motion source. The patch reapplied byte-for-byte. Scoped formatting, strict Clippy and the all-targets stock runner check passed.

Chrome `158.0.8127.37` used the Apple Metal 3 WebGPU adapter with fallback disabled. The scoped runner passed M256–M262 in 13.9 seconds. The independent checker confirmed 542 checks, 27 PNGs, zero page errors and zero test-owned processes after exit. The focused entry uses the same assertions as the stock Material case. The full stock browser suite did not run in this lane.

## Static cases

The unmodified M256–M261 runner passed all 369 checks against the animated Gallery. The old independent image check caught its first Basic screenshot during the final part of entry instead of holding at full opacity. [Initial diagnostic](focused.TPJy4bX1/run.KuBz9cwW/initial-static-verifier-diagnosis.json) · [Expected image-verifier rejection](focused.TPJy4bX1/run.KuBz9cwW/old-static-verifier.log).

The amended static captures wait until the appropriate panel is fully opaque. All existing geometry, keyboard, accessibility and pixel comparisons stay in place. Their pinned ink and unclipped description masks now pass against the animated Gallery.

## Actual motion and input

The test sets a controlled monotonic value for `performance.now()` while Chrome continues running real animation frames. The normal Basic dialog is already modal at zero opacity. At 40 ms its entry opacity is 0.475069, and the captured panel and scrim colors lie between the measured endpoints. Escape restores the opener and removes the retired controls from Chrome AX before the visual panel disappears. At 40 ms into exit, opacity is 0.524931. A trusted pointer click through the still-painted scrim reopens the panel from that exact intermediate paint. [Entry](focused.3br5IONC/run.XG6PBBHs/material-dialog-adaptive-motion-basic-entering.png) · [Exit](focused.3br5IONC/run.XG6PBBHs/material-dialog-adaptive-motion-basic-exiting.png) · [Reversal](focused.3br5IONC/run.XG6PBBHs/material-dialog-adaptive-motion-basic-reversed.png).

The nested Discard prompt's scrim continues darkening the editor during exit. Browser focus and accessibility have already returned to the editor. The test inserts `！` into its draft while the outgoing prompt still paints. The resolved draft is `秋日散步！` and the saved value stays unchanged. [Nested exit](focused.3br5IONC/run.XG6PBBHs/material-dialog-adaptive-motion-nested-exiting.png).

With Chrome's Reduce Motion media preference emulated, the same control snaps to the exact settled open and closed colors. The test did not advance the frozen clock in this profile. [Reduced open](focused.3br5IONC/run.XG6PBBHs/material-dialog-adaptive-motion-basic-reduced-open.png) · [Reduced closed](focused.3br5IONC/run.XG6PBBHs/material-dialog-adaptive-motion-basic-reduced-closed.png).

[Raw report](focused.3br5IONC/run.XG6PBBHs/browser-material-dialog.json) · [Independent verifier](focused.3br5IONC/run.XG6PBBHs/verified-dialog.json) · [Runner log](focused.3br5IONC/run.XG6PBBHs/runner.log) · [Stock lint log](stock-source-check.log) · [Evidence seal](../SHA256SUMS)

## Replay

The pinned Gallery Wasm is `75bbd6262ec028ce8152e3f53604c3ad4297ec8eb33d9b0e1768f2ec9eccfb2c`. To rebuild it, run `cap cargo xtask web build material-gallery --release --no-compress --out "$PWD/../web/material-gallery"` inside `source` with the usual Command Line Tools SDK, offline Cargo, two build jobs and a fresh target under `/tmp/cargo-target`.

```bash
python3 -B prepare-focused.py
pin=$(mktemp -d "$(pwd -P)/focused.XXXXXXXX")
bash build-focused.sh "$pin"
bash run-focused.sh "$pin"
python3 -B verify-motion.py "$pin"/run.*
```

Chrome runs with `--headless=new`. The verifier uses Pillow 12 or later. The browser proof covers actual rendered pixels, trusted browser pointer and key input, and Chrome accessibility. It does not exercise live system preferences, speech, or a Full-screen FAB container transform.

Start with the [572-file Basic Motion source](../native/). Apply the complete M256–M262 patch in a scratch source subdirectory and verify its 574 inputs before building. Copy the replay scripts to the scratch root.
