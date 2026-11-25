# Surface v0.1.58 Common Buttons evidence

The 656-file candidate matches tree `8ba487496a2816f3bff3363328ba0dc40a059c30`. The patch applies to v0.1.57 tree `72cf21a76d284e3a299aedff91b953eae232db07`.
The source manifest SHA-256 is `b03623198811629a68a2dfe5620d5705c55ea7744157ff2ac732d174833eedbf`.

- The workspace gate passed 1,492 Rust tests and three Wasm checks. The GPU gate ran 16 standard and 2 supplemental offscreen tests on Apple M1 Max Metal.
- Two native headless replays each saved five PNGs. The captures match byte for byte across Page.screenshot, CLI and MCP and across both replays. 16 samples found zero visible windows.
- The five stock macOS suites passed 23,004 checks through M319. Chrome used Apple Metal. The observer found zero owned windows and zero surviving processes. The package includes seven original Canvas PNGs and the typed Common Buttons report.
- Both stock IME cancellation reports and raw reports for Standard, Forms, Range, Vertical Centered and Inset Icon Sliders support the retained checks.

## Recheck in place

Use Python 3.12 or newer with Pillow 12.2 or newer. Supply the reviewed package manifest SHA-256.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-common-buttons.py --verify-package . --package-sha "$reviewed_seal"
```

The offline command checks the sealed contents, compares the native sampled pixels and replays the stock Common Buttons pixel checks. It opens no app. The compact package keeps the stock verdict and selected raw reports. The original run keeps the full browser artifact inventory. The source patch and test logs decompress to their original bytes.

## Reproduce from source

Apply the decompressed patch to the exact base tree. Use the committed dependency lockfile and the standard Surface build environment.

```sh
gzip -dc surface-v0.1.58.patch.gz > /tmp/surface-v0.1.58.patch
git apply --check --whitespace=error-all /tmp/surface-v0.1.58.patch
git apply --whitespace=error-all /tmp/surface-v0.1.58.patch
shasum -a 256 -c /path/to/evidence/source.sha256
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/cargo-target/surface-common-buttons-repro
export SURFACE_GPU_EVIDENCE=/tmp/surface-common-buttons-repro/gpu-offscreen.png
sh scripts/check.sh
sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --lib gpu_offscreen_raster_cache -- --ignored --nocapture
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
```

The native harness README records the replay commands. The stock invocation records the five case arguments and before/after input pins. See [stock reproduction](stock-macos/reproduction.md) for the guarded replay. The full offline stock helper requires the original artifact inventory, runtime bundle and predecessor tools.

The evidence covers the local Mac. The native run used keyboard and programmatic input. Chrome used trusted keyboard, pointer and touch events and checked browser accessibility semantics, actual spring frames and the Reduce Motion endpoint. Hosted Linux, live screen-reader speech and system accessibility settings remain open.
