# Surface v0.1.57 Slider Refinements evidence

The 652-file R1 candidate matches tree `72cf21a76d284e3a299aedff91b953eae232db07`. The patch applies to v0.1.56 tree `6e7c09a6eda249eb046babf3f2a4b392b0aaa99c`.
The source manifest SHA-256 is `5936ee7ecfdf584a2ca3acf7fa3297db09e86132b8a5dd60bd169391a90d44c1`.

- The workspace gate passed 1,481 Rust tests and three Wasm checks. The GPU gate ran 16 standard and 2 supplemental offscreen tests on Apple M1 Max Metal.
- Two native headless replays each saved eight PNGs. The captures match byte for byte across Page.screenshot, CLI and MCP and across both replays. 32 samples found zero visible windows.
- The five stock macOS suites passed 22,606 checks through M315. Chrome used Apple Metal. The observer found zero owned windows and zero surviving processes. The package includes seven original Canvas PNGs and the two typed .57 reports.

## Recheck in place

Use Python 3.12 or newer with Pillow 12.2 or newer. Supply the reviewed package manifest SHA-256.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-slider-refinements.py --verify-package . --package-sha "$reviewed_seal"
```

The offline command checks the sealed contents, compares the native sampled pixels and replays the stock Slider Refinements pixel checks. It opens no app. The compact package keeps the stock verdict and selected raw reports. The original run keeps the full browser artifact inventory. The source patch and test logs decompress to their original bytes.

## Reproduce from source

Apply the decompressed patch to the exact base tree. Use the committed dependency lockfile and the standard Surface build environment.

```sh
gzip -dc surface-v0.1.57.patch.gz > /tmp/surface-v0.1.57.patch
git apply --check --whitespace=error-all /tmp/surface-v0.1.57.patch
git apply --whitespace=error-all /tmp/surface-v0.1.57.patch
shasum -a 256 -c /path/to/evidence/source.sha256
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/cargo-target/surface-slider-refinements-repro
export SURFACE_GPU_EVIDENCE=/tmp/surface-slider-refinements-repro/gpu-offscreen.png
sh scripts/check.sh
sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --lib gpu_offscreen_raster_cache -- --ignored --nocapture
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
```

The native harness README records the replay commands. The stock invocation records the five case arguments and before/after input pins. See [stock reproduction](stock-macos/reproduction.md) for the guarded replay. The full offline stock helper requires the original artifact inventory, runtime bundle and predecessor tools.

The evidence covers the local Mac. The native run used keyboard and programmatic input. Chrome used trusted keyboard, pointer and touch events and checked browser accessibility semantics. Hosted Linux, live screen-reader speech and system accessibility settings remain open.
