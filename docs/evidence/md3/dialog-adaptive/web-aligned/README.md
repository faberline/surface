# Integrated Dialog RTL browser verification

The [aligned input](aligned-source.sha256) contains 565 files. SHA-256 `d038f7b00b6aab24933bd3d047b536d4d707e54c3b8285a628ed037c20fb0229`. Applying the [original M256–M260 patch](dialog-web-e2e.patch) produced the [566-file baseline](baseline-browser-source.sha256), SHA-256 `9be494d46193b8c0b2c007c941560fc6c9b82f114a8237c88b76710780211bbb`. The five focused cases passed 277 checks with seven PNGs, zero page errors and zero owned processes. [Baseline receipt](focused.jxz0iZs3/run.fUu6jBJF/verified-dialog.json).

Rebuild the [565-file aligned input](../alignment/README.md#reproduce-the-native-replay). The [combined browser patch](aligned-dialog-e2e.patch) applies to that source. The [M261-only patch](m261-rtl-e2e.patch) applies if M256–M260 already exist. Only `e2e/src/material.rs` and `e2e/src/material_dialog.rs` differ from the aligned input. The [566-file final manifest](source.sha256) is `351c1727ba3e04269de675a513c565a11e804a82829003fca1e5b7294894f2b3`. The combined patch applied byte-for-byte in a separate check. Scoped formatting, strict Clippy and the all-target `surface-e2e` check passed. Product source and motion are unchanged.

Chrome `158.0.8127.37` used the nonfallback Apple Metal 3 WebGPU adapter. The scoped test runner passed M256–M261 in 8.1 sec, with 369 checks, 11 PNGs, zero page errors and zero owned processes after exit. The focused entry ran the same Rust checks as the stock journey. The full Material journey did not run in this lane.

## M261 actual pixels and Chrome accessibility

At 320×400 dp in Dark with RTL direction and 200% text, the two unequal Full-screen title lines end at x249.5 dp. The prior native Metal replay measured x249 dp for both lines at 1×. [Wrapped editor title](focused.E8DAbV1v/run.YWpMGIbj/material-dialog-adaptive-rtl-200-edit.png).

The dirty confirmation scrolls its support inside a 32 dp body. Trusted wheel input moves the support by 48 dp. Its final line has zero visible ink before scrolling and 2,741 near-foreground pixels after it. The entire relative ink mask matches the unclipped reference at 320×520 dp. Heading and both action bounds stay fixed. Independent comparison of the images finds 11,413 heading pixels, 5,799 Continue pixels and 3,250 Discard pixels in the same places. [Before wheel](focused.E8DAbV1v/run.YWpMGIbj/material-dialog-adaptive-rtl-200-discard-before.png) · [After wheel](focused.E8DAbV1v/run.YWpMGIbj/material-dialog-adaptive-rtl-200-discard-after.png) · [Unclipped reference](focused.E8DAbV1v/run.YWpMGIbj/material-dialog-adaptive-rtl-200-discard-unclipped.png).

The Chrome accessibility tree exposes `未儲存的行程變更會遺失。` in full before and after scrolling. The real DOM link resolves to the same text. Continue stays focused through wheel and resize, then returns focus to the editor with the draft preserved.

With the same runner and the earlier unaligned Gallery, M256–M260 still passed and M261 rejected the unequal right edges. The two lines had right edges at pixels 259 and 83 inside the 320-pixel title slot. The aligned edges are both 315. [Expected-failure log](focused.E8DAbV1v/unaligned.88S18pyM/runner.log) · [Old unaligned title](focused.E8DAbV1v/unaligned.88S18pyM/material-dialog-adaptive-rtl-200-edit.png).

[Raw report](focused.E8DAbV1v/run.YWpMGIbj/browser-material-dialog.json) · [Independent receipt](focused.E8DAbV1v/run.YWpMGIbj/verified-dialog.json) · [Runner log](focused.E8DAbV1v/run.YWpMGIbj/runner.log) · [Stock lint receipt](stock-source-check.log) · [Combined checksums](SHA256SUMS)

## Reproduce

Build in a fresh scratch directory. Copy the proof files and scripts into that directory. Put the verified 565-file aligned source in its `source` subdirectory and apply `aligned-dialog-e2e.patch` with `git apply --whitespace=error-all`. Verify `source.sha256` from the source subdirectory. Build the Web bundle inside `source` using `CARGO_BUILD_JOBS=2 cap cargo xtask web build material-gallery --release --no-compress --out "$PWD/../web/material-gallery"`. Use a fresh Cargo target under `/tmp`. The recorded Wasm SHA-256 is `9489031a5525bc9eadd3297f56bce104ac098e5126ab761a218dd9a59f02cc37`.

```bash
python3 -B prepare-focused.py
pin=$(mktemp -d "$(pwd -P)/focused.XXXXXXXX")
bash build-focused.sh "$pin"
bash run-focused.sh "$pin"
python3 -B verify-integrated.py "$pin"/run.*
```

All browser runs use `--headless=new`. The independent image check uses Pillow 12 or later. This lane used browser focus and accessibility metadata. Live VoiceOver, live operating-system composition and motion timing are separate checks.

## Coverage record

The [docs overlay](dialog-docs.patch) updates the Dialog row and API guide. It applies after the aligned and browser patches. The row stays partial while motion and live-platform checks are open.
