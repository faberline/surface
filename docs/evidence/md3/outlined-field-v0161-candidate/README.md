# Surface v0.1.61 Outlined Text Field R2 evidence

The 668-file candidate matches tree `7c7881f78c3dc44129f52688a079b6874c9a4991`. The patch applies to v0.1.60 tree `b9539a68d5993b8c29a9dc17da571b31078c6874`.
The source manifest SHA-256 is `3c79ae5ee15bf3b9b786d37452f1e0e5929ac0d31276ca7f6506897462dd9cc8`.
R2 changes only the Outlined Text Field integration test from R1. The
R1 workspace failed its stale expectation for the assertive status
child. The unchanged R1 focused and native reports are calibration
under `calibration-r1`. The R2 counts below come from separate exact runs.

- The workspace gate passed 1,528 Rust tests and three Wasm checks. The GPU gate ran 16 standard and 2 supplemental offscreen tests on Apple M1 Max Metal.
- Two native headless replays each saved four PNGs. The captures match byte for byte across Page.screenshot, CLI and MCP and across both replays. 16 samples found zero visible windows.
- Native wide and compact frames verify the 1 dp and 3 dp outlines, transparent notch, closed long-label outline and disabled text ink against both original replays.
- The five stock macOS suites passed 23,656 checks through M328. Chrome used Apple Metal. The observer found zero owned windows and zero surviving processes. The package includes five original stock Canvas PNGs and the typed Outlined Text Field report.
- The separate R2 focused run passed 178 checks in one case. It supplies five scoped PNGs with fresh guard receipts and the same R2 Gallery Web bytes as stock. It used the pinned R1 focused runner because all E2E source bytes match. The first trusted Enter did not submit during CDP-simulated composition. The report records whether Chrome used an explicit synthetic text commit before the separate trusted Enter submitted.
- Both stock IME cancellation reports and raw reports for Standard, Forms, Range, Vertical Centered and Inset Icon Sliders support the retained checks.

## Recheck in place

Use Python 3.12 or newer with Pillow 12.2 or newer. Supply the reviewed package manifest SHA-256.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-outlined-field.py --verify-package . --package-sha "$reviewed_seal"
```

The offline command checks the sealed contents, compares the native sampled pixels and replays the stock Outlined Text Field pixel checks. It opens no app. The compact package keeps the stock verdict and selected raw reports. The original run keeps the full browser artifact inventory. The source patch and test logs decompress to their original bytes.

## Reproduce from source

Apply the decompressed patch to the exact base tree. Use the committed dependency lockfile and the standard Surface build environment.

```sh
gzip -dc surface-v0.1.61.patch.gz > /tmp/surface-v0.1.61.patch
git apply --check --whitespace=error-all /tmp/surface-v0.1.61.patch
git apply --whitespace=error-all /tmp/surface-v0.1.61.patch
shasum -a 256 -c /path/to/evidence/source.sha256
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/cargo-target/surface-outlined-field-repro
export SURFACE_GPU_EVIDENCE=/tmp/surface-outlined-field-repro/gpu-offscreen.png
sh scripts/check.sh
sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --lib gpu_offscreen_raster_cache -- --ignored --nocapture
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
```

The native harness README records the replay commands. The stock invocation records the five case arguments and before/after input pins. See [stock reproduction](stock-macos/reproduction.md) for the guarded replay. The full offline stock helper requires the original artifact inventory, runtime bundle and predecessor tools.

The evidence covers the local Mac. The native run used keyboard and programmatic input. Chrome used trusted keyboard, pointer and touch events and checked browser accessibility semantics, measured outlines, linked error descriptions, disabled and read-only editors, and dark RTL at 200 percent text. Hosted Linux, live screen-reader speech and system accessibility settings remain open.
