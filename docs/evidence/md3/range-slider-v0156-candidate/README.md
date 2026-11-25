# Surface v0.1.56 Range Slider evidence

The 646-file candidate matches tree `6e7c09a6eda249eb046babf3f2a4b392b0aaa99c`. The patch applies to v0.1.55 tree `18bf4098eefd03ff562e058ceb5d7f99a34447a2`.
The source manifest SHA-256 is `de92395ee2ab7b8d7d49760adb2722d5b44ca270c2002deb6fefbecd809e8647`.

- The workspace gate passed 1,464 Rust tests and three Wasm checks. The GPU gate ran 16 offscreen tests on Metal.
- Two native headless replays each saved five PNGs. The captures match byte for byte across Page.screenshot, CLI and MCP and across both replays. 20 samples found zero visible windows.
- The five stock macOS suites passed 22,116 checks through M309. Chrome used Apple Metal. The observer found zero owned windows and zero surviving processes. Four original Range Slider PNGs are included.
- The separate focused run passed 875 checks through M298–M309. Its development receipts support diagnosis. The stock run supplies release evidence.

## Recheck in place

Use Python 3.12 or newer with Pillow 12.2 or newer. Supply the reviewed package manifest SHA-256.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-range-slider.py --verify-package . --package-sha "$reviewed_seal"
```

The offline command checks the sealed contents, compares the native sampled pixels and replays the stock Range Slider pixel checks. It opens no app. The compact package keeps the stock verdict and selected raw reports. The original run keeps the full browser artifact inventory. The source patch and test logs decompress to their original bytes.

## Reproduce from source

Apply the decompressed patch to the exact base tree. Use the committed dependency lockfile and the standard Surface build environment.

```sh
gzip -dc surface-v0.1.56.patch.gz > /tmp/surface-v0.1.56.patch
git apply --check --whitespace=error-all /tmp/surface-v0.1.56.patch
git apply --whitespace=error-all /tmp/surface-v0.1.56.patch
shasum -a 256 -c /path/to/evidence/source.sha256
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/cargo-target/surface-slider-forms-repro
export SURFACE_GPU_EVIDENCE=/tmp/surface-slider-forms-repro/gpu-offscreen.png
sh scripts/check.sh
sh scripts/check-gpu.sh
```

The native harness README records the exact command and independent repeats. The stock invocation and inputs record the five case arguments and preflight and postflight pins. The local evidence does not cover Linux hosted CI, actual screen-reader speech or system accessibility settings.
