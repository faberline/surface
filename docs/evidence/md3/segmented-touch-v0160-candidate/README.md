# Surface v0.1.60 Segmented Button touch evidence

The 662-file candidate matches tree `b9539a68d5993b8c29a9dc17da571b31078c6874`. The patch applies to v0.1.59 tree `757179e260a331b4488c6be1b49410263097d675`.
The source manifest SHA-256 is `4874c29831d6a87a60c1e3b98071cc522d2ab9a933deab60b6f33091bcfd25b9`.

- The workspace gate passed 1,515 Rust tests and three Wasm checks. The GPU gate ran 16 standard and 2 supplemental offscreen tests on Apple M1 Max Metal.
- Two native headless replays each saved four PNGs. The captures match byte for byte across Page.screenshot, CLI and MCP and across both replays. 24 samples found zero visible windows.
- The 320 dp native frame shows the 18 dp icon centered above a complete two-line label, with an 8 dp gap. The verifier rechecks the icon and label pixels against the original in each replay.
- The five stock macOS suites passed 23,478 checks through M324. Chrome used Apple Metal. The observer found zero owned windows and zero surviving processes. The package includes seven original stock Canvas PNGs and the typed Segmented Button report, including both touch-list directions, vertical handoff and trusted capture loss.
- The separate R3 focused run passed 474 checks in one case. It supplies seven development PNGs. Its runner, bundle and guard receipt remain under `focused`.
- Both stock IME cancellation reports and raw reports for Standard, Forms, Range, Vertical Centered and Inset Icon Sliders support the retained checks.

## Recheck in place

Use Python 3.12 or newer with Pillow 12.2 or newer. Supply the reviewed package manifest SHA-256.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-segmented-touch.py --verify-package . --package-sha "$reviewed_seal"
```

The offline command checks the sealed contents, compares the native sampled pixels and replays the stock Baseline Segmented Button pixel checks. It opens no app. The compact package keeps the stock verdict and selected raw reports. The original run keeps the full browser artifact inventory. The source patch and test logs decompress to their original bytes.

## Reproduce from source

Apply the decompressed patch to the exact base tree. Use the committed dependency lockfile and the standard Surface build environment.

```sh
gzip -dc surface-v0.1.60.patch.gz > /tmp/surface-v0.1.60.patch
git apply --check --whitespace=error-all /tmp/surface-v0.1.60.patch
git apply --whitespace=error-all /tmp/surface-v0.1.60.patch
shasum -a 256 -c /path/to/evidence/source.sha256
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/cargo-target/surface-segmented-repro
export SURFACE_GPU_EVIDENCE=/tmp/surface-segmented-repro/gpu-offscreen.png
sh scripts/check.sh
sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --lib gpu_offscreen_raster_cache -- --ignored --nocapture
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
```

The native harness README records the replay commands. The stock invocation records the five case arguments and before/after input pins. See [stock reproduction](stock-macos/reproduction.md) for the guarded replay. The full offline stock helper requires the original artifact inventory, runtime bundle and predecessor tools.

The evidence covers the local Mac. The native run used keyboard and programmatic input. Chrome used trusted keyboard, pointer and touch events and checked browser accessibility semantics, each seam, touch padding, and the two RTL focus paths. Hosted Linux, live screen-reader speech and system accessibility settings remain open.
