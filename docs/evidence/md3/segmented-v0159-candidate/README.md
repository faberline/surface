# Surface v0.1.59 Baseline Segmented Button evidence

The 660-file candidate matches tree `757179e260a331b4488c6be1b49410263097d675`. The patch applies to v0.1.58 tree `8ba487496a2816f3bff3363328ba0dc40a059c30`.
The source manifest SHA-256 is `9051c16e996c9b310062682765a7d0bc54cf42dfbadbda4c5f63bd457b5d53b8`.

- The workspace gate passed 1,502 Rust tests and three Wasm checks. The GPU gate ran 16 standard and 2 supplemental offscreen tests on Apple M1 Max Metal.
- Two native headless replays each saved four PNGs. The captures match byte for byte across Page.screenshot, CLI and MCP and across both replays. 24 samples found zero visible windows.
- The five stock macOS suites passed 23,315 checks through M323. Chrome used Apple Metal. The observer found zero owned windows and zero surviving processes. The package includes five original Canvas PNGs and the typed Baseline Segmented Button report.
- Both stock IME cancellation reports and raw reports for Standard, Forms, Range, Vertical Centered and Inset Icon Sliders support the retained checks.

## Recheck in place

Use Python 3.12 or newer with Pillow 12.2 or newer. Supply the reviewed package manifest SHA-256.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-segmented.py --verify-package . --package-sha "$reviewed_seal"
```

The offline command checks the sealed contents, compares the native sampled pixels and replays the stock Baseline Segmented Button pixel checks. It opens no app. The compact package keeps the stock verdict and selected raw reports. The original run keeps the full browser artifact inventory. The source patch and test logs decompress to their original bytes.

## Reproduce from source

Apply the decompressed patch to the exact base tree. Use the committed dependency lockfile and the standard Surface build environment.

```sh
gzip -dc surface-v0.1.59.patch.gz > /tmp/surface-v0.1.59.patch
git apply --check --whitespace=error-all /tmp/surface-v0.1.59.patch
git apply --whitespace=error-all /tmp/surface-v0.1.59.patch
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
